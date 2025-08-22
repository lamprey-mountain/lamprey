import { createEffect, createSignal, Show, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import { getUrl } from "../media/util.tsx";
import { createUpload } from "sdk";
import { useApi } from "../api.tsx";

export function Info(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();

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

	const api = useApi();
	const setAvatar = async (f: File) => {
		if (f) {
			await createUpload({
				client: api.client,
				file: f,
				onComplete(media) {
					api.client.http.PATCH("/api/v1/room/{room_id}", {
						params: { path: { room_id: props.room.id } },
						body: { icon: media.id },
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
					ctx.client.http.PATCH("/api/v1/room/{room_id}", {
						params: { path: { room_id: props.room.id } },
						body: { icon: null },
					});
				},
			});
		}
	};

	const [editingName, setEditingName] = createSignal(props.room.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.room.description,
	);

	const save = () => {
		ctx.client.http.PATCH("/api/v1/room/{room_id}", {
			params: { path: { room_id: props.room.id } },
			body: { name: editingName(), description: editingDescription() },
		});
	};

	return (
		<>
			<h2>info</h2>
			<button onClick={save}>save changes</button>
			<br />
			name
			<br />
			<input
				value={editingName()}
				type="text"
				onInput={(e) => setEditingName(e.target.value)}
			/>
			<br />
			<br />
			description
			<br />
			<textarea onInput={(e) => setEditingDescription(e.target.value)}>
				{editingDescription()}
			</textarea>
			<br />
			<br />
			<div>
				room avatar (click to upload):
				<Show
					when={props.room.icon}
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
						src={getThumb(props.room.icon!)}
						class="avatar"
					/>
				</Show>
				<input
					style="display:none"
					ref={avatarInputEl}
					type="file"
					onInput={(e) => {
						const f = e.target.files?.[0];
						if (f) setAvatar(f);
					}}
				/>
			</div>
			<div>
				room id: <code class="select-all">{props.room.id}</code>
			</div>
			<br />
			<div>(todo) visibility</div>
			<div>(todo) order, layout</div>
			<br />
			<div class="danger">
				<h3>danger zone</h3>
				<label>
					<button onClick={() => alert("todo")}>transfer ownership</button>
					<span style="margin-left:8px">
						makes this room someone else's problem
					</span>
				</label>
				<br />
				<label>
					<button onClick={() => alert("todo")}>archive</button>
					<span style="margin-left:8px">
						makes this entirely read-only and hides it in the nav bar
					</span>
				</label>
				<br />
			</div>
		</>
	);
}
