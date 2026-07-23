import type { EditorState } from "prosemirror-state";
import { createUpload } from "sdk";
import { createSignal, onMount, Show, type VoidProps } from "solid-js";
import { useApi, useChannels } from "@/api";
import { ChannelPicker } from "@/atoms/ChannelPicker";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { DurationInput } from "@/atoms/DurationInput";
import { Checkbox } from "@/atoms/icons";
import { Savebar } from "@/atoms/Savebar";
import { createEditor } from "@/components/features/editor/Editor.tsx";
import { RoomIcon } from "@/components/shared/User";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useModals } from "@/contexts/modal";
import type { ChannelT, RoomT } from "@/types";

// TODO: add welcome channel id config
// TODO: configure or remove room banner

export function Info(props: VoidProps<{ room: RoomT }>) {
	const [, modalCtl] = useModals();
	const channels = useChannels();

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
	const [editingAfkChannel, setEditingAfkChannel] =
		createSignal<ChannelT | null>(
			[...channels.cache.values()].find(
				(c) => c.id === props.room.afk_channel_id,
			) ?? null,
		);
	const [editingWelcomeChannel, setEditingWelcomeChannel] =
		createSignal<ChannelT | null>(
			[...channels.cache.values()].find(
				(c) => c.id === props.room.welcome_channel_id,
			) ?? null,
		);
	const [editingAfkTimeout, setEditingAfkTimeout] = createSignal(
		props.room.afk_channel_timeout / 1000,
	);
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
		editingPublic() !== props.room.public ||
		(editingAfkChannel()?.id ?? null) !== (props.room.afk_channel_id ?? null) ||
		(editingWelcomeChannel()?.id ?? null) !==
			(props.room.welcome_channel_id ?? null) ||
		editingAfkTimeout() * 1000 !== props.room.afk_channel_timeout;

	const getDescriptionFromState = () => {
		const state = editorState();
		if (!state) return "";
		return state.doc.textContent;
	};

	const save = () => {
		const description = getDescriptionFromState();
		const body: {
			name: string;
			public: boolean;
			description?: string;
			afk_channel_id?: string | null;
			welcome_channel_id?: string | null;
			afk_channel_timeout?: number;
		} = {
			name: editingName(),
			public: editingPublic(),
			afk_channel_id: editingAfkChannel()?.id ?? null,
			welcome_channel_id: editingWelcomeChannel()?.id ?? null,
			afk_channel_timeout: editingAfkTimeout() * 1000,
		};
		if (description.trim() !== "") {
			body.description = description;
		}
		api2.client.http.PATCH("/api/v1/room/{room_id}", {
			params: { path: { room_id: props.room.id } },
			body,
		});
	};

	const roomChannels = () =>
		[...channels.cache.values()].filter((c) => c.room_id === props.room.id);

	// FIXME: button to archive all threads
	const _archiveAllThreads = () => {
		modalCtl.confirm("really archive everything?", (confirmed) => {
			if (!confirmed) return;
			console.log(roomChannels());
			for (const thread of roomChannels()) {
				channels.archive(thread.id);
			}
		});
	};

	const reset = () => {
		setEditingName(props.room.name);
		setEditingDescription(props.room.description);
		setEditingPublic(props.room.public);
		setEditingAfkChannel(
			[...channels.cache.values()].find(
				(c) => c.id === props.room.afk_channel_id,
			) ?? null,
		);
		setEditingWelcomeChannel(
			[...channels.cache.values()].find(
				(c) => c.id === props.room.welcome_channel_id,
			) ?? null,
		);
		setEditingAfkTimeout(props.room.afk_channel_timeout / 1000);
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
							class="hidden"
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
			<div class="afk-settings">
				<div>
					<h3 class="dim">welcome channel</h3>
					{/* TODO: description? maybe as another column? <p class="dim">this is where user join messages will be sent</p> */}
					<ChannelPicker
						selected={editingWelcomeChannel()}
						channels={roomChannels}
						filter={(c) => c.type === "Text"}
						onSelect={(channel) => setEditingWelcomeChannel(channel ?? null)}
						placeholder="Select a channel..."
						required={false}
					/>
				</div>
			</div>
			<div class="afk-settings">
				<div>
					<h3 class="dim">afk channel</h3>
					<ChannelPicker
						selected={editingAfkChannel()}
						channels={roomChannels}
						filter={(c) => c.type === "Voice" || c.type === "Broadcast"}
						onSelect={(channel) => setEditingAfkChannel(channel ?? null)}
						placeholder="Select a channel..."
						required={false}
					/>
				</div>
				<div>
					<h3 class="dim">afk timeout</h3>
					<DurationInput
						onInput={(duration) => setEditingAfkTimeout(duration)}
						placeholder="select a duration..."
						presets={[
							{ label: "1 minute", seconds: 60 },
							{ label: "5 minutes", seconds: 60 * 5 },
							{ label: "10 minutes", seconds: 60 * 10 },
							{ label: "15 minutes", seconds: 60 * 15 },
							{ label: "1 hour", seconds: 60 * 60 },
						]}
						value={editingAfkTimeout()}
					/>
				</div>
			</div>
			<Savebar show={isDirty()} onCancel={reset} onSave={save} />
		</div>
	);
}
