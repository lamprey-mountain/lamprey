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

	const setAvatar = async (f: File) => {
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

	let avatarInputEl!: HTMLInputElement;

	const openAvatarPicker = () => {
		avatarInputEl?.click();
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
		<div class="user-settings-info">
			<h2>info</h2>
			<div class="box profile">
				<div
					class="name"
					onClick={setName}
				>
					{props.user.name}
				</div>
				<div class="description" onClick={setDescription}>
					{props.user.description}
				</div>
				<Show
					when={props.user.avatar}
					fallback={
						<div
							onClick={openAvatarPicker}
							class="avatar"
						>
						</div>
					}
				>
					<img
						onClick={openAvatarPicker}
						src={getThumb(props.user.avatar!)}
						class="avatar"
					/>
				</Show>
			</div>
			<div>
				id: <code class="select-all">{props.user.id}</code>
			</div>
			<input
				style="display:none"
				ref={avatarInputEl}
				type="file"
				onInput={(e) => {
					const f = e.target.files?.[0];
					if (f) setAvatar(f);
				}}
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
			<br />
			<label>
				<input
					type="checkbox"
					checked={ctx.settings.get("underline_links") === "yes"}
					onInput={toggle("underline_links")}
				/>{" "}
				always underline links
			</label>
			<br />
			<div class="danger">
				<h3>danger zone</h3>
				<label>
					<button onClick={() => alert("todo")}>self destruct</button>
					<span style="margin-left:8px">this will delete your account</span>
				</label>
			</div>
		</div>
	);
}
