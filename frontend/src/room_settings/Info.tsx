import { createEffect, createSignal, Show, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import { getThumbFromId, getUrl } from "../media/util.tsx";
import { createUpload } from "sdk";
import { useApi } from "../api.tsx";
import { Checkbox } from "../icons";
import { useModals } from "../contexts/modal";

export function Info(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();
	const [, modalCtl] = useModals();

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
			modalCtl.confirm("remove avatar?", (conf) => {
				if (!conf) return;
				ctx.client.http.PATCH("/api/v1/room/{room_id}", {
					params: { path: { room_id: props.room.id } },
					body: { icon: null },
				});
			});
		}
	};

	const [editingName, setEditingName] = createSignal(props.room.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.room.description,
	);
	const [editingPublic, setEditingPublic] = createSignal(props.room.public);

	const isDirty = () =>
		editingName() !== props.room.name ||
		editingDescription() !== props.room.description ||
		editingPublic() !== props.room.public;

	const save = () => {
		ctx.client.http.PATCH("/api/v1/room/{room_id}", {
			params: { path: { room_id: props.room.id } },
			body: {
				name: editingName(),
				description: editingDescription(),
				public: editingPublic(),
			},
		});
	};

	const threads = api.channels.list(() => props.room.id);
	const archiveAllThreads = () => {
		modalCtl.confirm("really archive everything?", (confirmed) => {
			if (!confirmed) return;
			console.log(threads());
			for (const thread of threads()?.items ?? []) {
				api.channels.archive(thread.id);
			}
		});
	};

	const reset = () => {
		setEditingName(props.room.name);
		setEditingDescription(props.room.description);
		setEditingPublic(props.room.public);
	};

	return (
		<>
			<h2>info</h2>
			<div class="dim">name</div>
			<input
				value={editingName()}
				type="text"
				onInput={(e) => setEditingName(e.target.value)}
			/>
			<br />
			<br />
			<div class="dim">description</div>
			<textarea
				value={editingDescription()}
				onInput={(e) => setEditingDescription(e.target.value)}
			/>
			<br />
			<br />
			{isDirty() && (
				<div class="savebar">
					<div class="inner">
						<div class="warning">you have unsaved changes</div>
						<button class="reset" onClick={reset}>
							cancel
						</button>
						<button class="save" onClick={save}>
							save
						</button>
					</div>
				</div>
			)}
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
			<label class="option">
				<input
					type="checkbox"
					checked={editingPublic()}
					onChange={(e) => setEditingPublic(e.target.checked)}
					style="display: none;"
				/>
				<Checkbox checked={editingPublic()} />
				<span>Make this room public (anyone can join and view)</span>
			</label>
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
