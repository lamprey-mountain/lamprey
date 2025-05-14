import { createSignal, Show, type VoidProps } from "solid-js";
import { createUpload, type User } from "sdk";
import { useCtx } from "../context.ts";
import { useApi } from "../api.tsx";
import { getUrl } from "../media/util.tsx";

export function Info(props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();

	const setName = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.PATCH("/api/v1/user/{user_id}", {
					params: { path: { user_id: "@self" } },
					body: { name },
				});
			},
		});
	};

	const setDescription = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "description?",
			cont(description) {
				if (typeof description !== "string") return;
				ctx.client.http.PATCH("/api/v1/user/{user_id}", {
					params: { path: { user_id: "@self" } },
					body: { description },
				});
			},
		});
	};

	const [file, setFile] = createSignal<File | null>(null);
	const setAvatar = async () => {
		const f = file();
		if (f) {
			await createUpload({
				client: api.client,
				file: f,
				onComplete(media) {
					api.client.http.PATCH("/api/v1/user/{user_id}", {
						params: { path: { user_id: "@self" } },
						body: { avatar: media.id },
					});
				},
				onFail(_error) {},
				onPause() {},
				onResume() {},
				onProgress(_progress) {},
			});
		} else {
			ctx.dispatch({
				do: "modal.confirm",
				text: "remove avatar?",
				cont(conf) {
					if (!conf) return;
					ctx.client.http.PATCH("/api/v1/user/{user_id}", {
						params: { path: { user_id: "@self" } },
						body: { avatar: null },
					});
				},
			});
		}
	};

	function getThumb(media_id: string) {
		const media = api.media.fetchInfo(() => media_id);
		const m = media();
		if (!m) return;
		const tracks = [m.source, ...m.tracks];
		const source =
			tracks.find((s) => s.type === "Thumbnail" && s.height === 64) ??
				tracks.find((s) => s.type === "Image");
		if (source) {
			return getUrl(source);
		} else {
			console.error("no valid avatar source?", m);
		}
	}

	const toggle = (setting: string) => () => {
		ctx.settings.set(
			setting,
			ctx.settings.get(setting) === "yes" ? "no" : "yes",
		);
	};

	return (
		<>
			<h2>info</h2>
			<div>name: {props.user.name}</div>
			<div>description: {props.user.description}</div>
			<Show when={props.user.avatar} fallback="avatar: none">
				<div>
					<div>avatar:</div>
					<img src={getThumb(props.user.avatar!)} class="avatar" />
				</div>
			</Show>
			<div>
				id: <code class="select-all">{props.user.id}</code>
			</div>
			<button onClick={setName}>set name</button>
			<br />
			<button onClick={setDescription}>set description</button>
			<br />
			<button onClick={setAvatar}>set avatar</button>
			<input
				type="file"
				onInput={(e) => setFile(e.target.files?.[0] ?? null)}
			/>
			<br />
			<h3>appearance (todo: move to separate section)</h3>
			<br />
			<label>
				<input
					type="checkbox"
					checked={ctx.settings.get("message_pfps") === "yes"}
					onInput={toggle("message_pfps")}
				/>{" "}
				show pfps in messages (experimental)
			</label>
			<label>
				<input
					type="checkbox"
					checked={ctx.settings.get("underline_links") === "yes"}
					onInput={toggle("underline_links")}
				/>{" "}
				always underline links
			</label>
		</>
	);
}
