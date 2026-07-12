import type { EditorState } from "prosemirror-state";
import { createUpload } from "sdk";
import { createSignal, onMount, Show, type VoidProps } from "solid-js";
import { useApi, useChannels } from "@/api";
import { useCtx } from "@/app/context";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { Savebar } from "@/atoms/Savebar";
import { createEditor } from "@/components/features/editor/Editor.tsx";
import { RoomIcon } from "@/components/shared/User";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useModals } from "@/contexts/modal";
import type { RoomT } from "@/types";

export function Info(props: VoidProps<{ room: RoomT }>) {
	const [, modalCtl] = useModals();

	let avatarInputEl!: HTMLInputElement;

	const api2 = useApi();
	const [roomIcon, setRoomIcon] = createSignal(props.room.icon);

	const setAvatarFile = async (f: File) => {
		await createUpload({
			client: api2.client,
			file: f,
			onComplete(media) {
				setRoomIcon(media.id);
				api2.client.http.PATCH("/api/v1/room/{room_id}", {
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
		await api2.client.http.PATCH("/api/v1/room/{room_id}", {
			params: { path: { room_id: props.room.id } },
			body: { icon: null },
		});
	};

	const openAvatarPicker = () => {
		avatarInputEl?.click();
	};

	const [editingName, setEditingName] = createSignal(props.room.name);
	const [_editingDescription, setEditingDescription] = createSignal(
		props.room.description,
	);
	const [editingPublic, setEditingPublic] = createSignal(props.room.public);
	const [editorState, setEditorState] = createSignal<EditorState | undefined>(
		undefined,
	);

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const editor = createEditor({
		channelId: () => props.room.id,
		roomId: () => props.room.id,
		toolbar,
		autocomplete,
		initialContent: () => props.room.description ?? "",
	});

	onMount(() => {
		const state = editorState();
		if (state) {
			editor.setState(state);
		}
	});

	const isDirty = () =>
		editingName() !== props.room.name ||
		getDescriptionFromState() !== props.room.description ||
		editingPublic() !== props.room.public;

	const getDescriptionFromState = () => {
		const state = editorState();
		if (!state) return "";
		return state.doc.textContent;
	};

	const save = () => {
		const description = getDescriptionFromState();
		const body: { name: string; public: boolean; description?: string } = {
			name: editingName(),
			public: editingPublic(),
		};
		if (description.trim() !== "") {
			body.description = description;
		}
		api2.client.http.PATCH("/api/v1/room/{room_id}", {
			params: { path: { room_id: props.room.id } },
			body,
		});
	};

	const channels2 = useChannels();
	const threads = () =>
		[...channels2.cache.values()].filter((c) => c.room_id === props.room.id);
	const _archiveAllThreads = () => {
		modalCtl.confirm("really archive everything?", (confirmed) => {
			if (!confirmed) return;
			console.log(threads());
			for (const thread of threads()) {
				channels2.archive(thread.id);
			}
		});
	};

	const reset = () => {
		setEditingName(props.room.name);
		setEditingDescription(props.room.description);
		setEditingPublic(props.room.public);
	};

	return (
		<div class="room-settings-info">
			<h2>info</h2>
			<div class="room-profile">
				<label class="room-icon">
					<h3 class="dim">icon</h3>
					<div class="avatar-uploader" onClick={openAvatarPicker}>
						<div class="avatar-inner">
							<RoomIcon room={props.room} />
							<div class="overlay">upload avatar</div>
						</div>
						<Show when={roomIcon()}>
							<button
								type="button"
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
				</label>
				<div class="name-description">
					<label class="name">
						<h3 class="dim">name</h3>
						<input
							value={editingName()}
							type="text"
							class="name-input"
							onInput={(e) => setEditingName(e.target.value)}
						/>
					</label>
					<label class="description">
						<h3 class="dim">description</h3>
						<editor.View
							onChange={(state) => setEditorState(state)}
							placeholder="room description..."
							submitOnEnter={false}
							autofocus={false}
						/>
					</label>
				</div>
			</div>
			<br />
			<CheckboxOption
				id={`room-${props.room.id}-public`}
				checked={editingPublic()}
				onChange={(checked) => setEditingPublic(checked)}
				seed={`room-${props.room.id}-public`}
			>
				<Checkbox
					checked={editingPublic()}
					seed={`room-${props.room.id}-public`}
				/>
				<div>
					<div>Make this room public</div>
					<div class="dim">anyone can join and view</div>
				</div>
			</CheckboxOption>
			<Savebar show={isDirty()} onCancel={reset} onSave={save} />
		</div>
	);
}
