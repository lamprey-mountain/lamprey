import { createEffect, createSignal, Show, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import { getThumbFromId, getUrl } from "../media/util.tsx";
import { createUpload } from "sdk";
import { useApi } from "../api.tsx";
import { Checkbox } from "../icons";
import { useModals } from "../contexts/modal";
import { RoomIcon } from "../User.tsx";

export function Info(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();
	const [, modalCtl] = useModals();

	let avatarInputEl!: HTMLInputElement;

	const api = useApi();
	const [roomIcon, setRoomIcon] = createSignal(props.room.icon);

	const setAvatarFile = async (f: File) => {
		await createUpload({
			client: api.client,
			file: f,
			onComplete(media) {
				setRoomIcon(media.id);
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
	};

	const removeAvatar = async () => {
		setRoomIcon(null);
		await api.client.http.PATCH("/api/v1/room/{room_id}", {
			params: { path: { room_id: props.room.id } },
			body: { icon: null },
		});
	};

	const openAvatarPicker = () => {
		avatarInputEl?.click();
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
				<div class="avatar-uploader" onClick={openAvatarPicker}>
					<div class="avatar-inner">
						<RoomIcon room={props.room} />
						<div class="overlay">upload avatar</div>
					</div>
					<Show when={roomIcon()}>
						<button
							class="remove"
							onClick={(e) => {
								e.stopPropagation();
								removeAvatar();
							}}
						>
							remove
						</button>
					</Show>
					<input
						style="display:none"
						ref={avatarInputEl}
						type="file"
						onInput={(e) => {
							const f = e.target.files?.[0];
							if (f) setAvatarFile(f);
						}}
					/>
				</div>
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
		</>
	);
}
