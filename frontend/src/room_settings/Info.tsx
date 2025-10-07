import { createEffect, createSignal, Show, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import { getThumbFromId, getUrl } from "../media/util.tsx";
import { createUpload } from "sdk";
import { useApi } from "../api.tsx";

export function Info(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();

	let avatarInputEl!: HTMLInputElement;
	const openAvatarPicker = () => {
		avatarInputEl?.click();
	};

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

	const threads = api.threads.list(() => props.room.id);
	const archiveAllThreads = () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "really archive everything?",
			cont(confirmed) {
				if (!confirmed) return;
				console.log(threads());
				for (const thread of threads()?.items ?? []) {
					ctx.client.http.PUT("/api/v1/thread/{thread_id}/archive", {
						params: { path: { thread_id: thread.id } },
					});
				}
			},
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
						src={getThumbFromId(props.room.icon!, 64)}
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
			<ul style="list-style: disc inside">
				<li>
					room id: <code class="select-all">{props.room.id}</code>
				</li>
				<li>
					owner id: <code class="select-all">{props.room.owner_id}</code>
				</li>
			</ul>
			<br />
			<div>(todo) visibility</div>
			<div>(todo) order, layout</div>
			<br />
			<div class="danger">
				<h3>danger zone</h3>
				<label>
					<button onClick={archiveAllThreads}>archive all threads</button>
					<span style="margin-left:8px">
						archive all threads in this room
					</span>
				</label>
				<br />
				<label>
					<button onClick={() => alert("todo")}>transfer ownership</button>
					<span style="margin-left:8px">
						makes this room someone else's problem
					</span>
				</label>
				<br />
				<label>
					<button onClick={() => alert("todo")}>obliterate</button>
					<span style="margin-left:8px">
						delete this room
					</span>
				</label>
				<br />
			</div>
		</>
	);
}
