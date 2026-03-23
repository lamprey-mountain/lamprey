import type { Channel } from "sdk";
import { createSignal, For, onMount, Show, type VoidProps } from "solid-js";
import { useCtx } from "../../../context.ts";
import { useApi, useChannels2 } from "@/api";
import { useModals } from "../../../contexts/modal";
import { Checkbox } from "../../../icons";
import {
	DurationInput,
	type DurationPreset,
} from "../../../atoms/DurationInput.tsx";
import { createUpload } from "sdk";
import { ChannelIconGdm } from "../../../User.tsx";
import { Savebar } from "../../../atoms/Savebar";
import { CheckboxOption } from "../../../atoms/CheckboxOption";
import { createEditor } from "../editor/Editor.tsx";
import { EditorState } from "prosemirror-state";

const slowmodePresets: DurationPreset[] = [
	{ label: "disabled", seconds: null as any },
	{ label: "1 second", seconds: 1 },
	{ label: "2 seconds", seconds: 2 },
	{ label: "3 seconds", seconds: 3 },
	{ label: "5 seconds", seconds: 5 },
	{ label: "10 seconds", seconds: 10 },
	{ label: "15 seconds", seconds: 15 },
	{ label: "30 seconds", seconds: 30 },
	{ label: "1 minute", seconds: 60 },
	{ label: "2 minutes", seconds: 120 },
	{ label: "5 minutes", seconds: 300 },
	{ label: "10 minutes", seconds: 600 },
	{ label: "15 minutes", seconds: 900 },
	{ label: "1 hour", seconds: 3600 },
	{ label: "2 hours", seconds: 7200 },
	{ label: "6 hours", seconds: 21600 },
	{ label: "24 hours", seconds: 86400 },
];

export function Info(props: VoidProps<{ channel: Channel }>) {
	const ctx = useCtx();
	const api = useApi();
	const channels2 = useChannels2();
	const [, modalctl] = useModals();
	const [editingNsfw, setEditingNsfw] = createSignal(props.channel.nsfw);
	const [editingName, setEditingName] = createSignal(props.channel.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.channel.description,
	);
	const [editingSlowmodeMessage, setEditingSlowmodeMessage] = createSignal(
		props.channel.slowmode_message,
	);
	const [editingSlowmodeThread, setEditingSlowmodeThread] = createSignal(
		props.channel.slowmode_thread,
	);
	const [editingDefaultSlowmodeMessage, setEditingDefaultSlowmodeMessage] =
		createSignal(
			props.channel.default_slowmode_message,
		);
	const [editingUserLimit, setEditingUserLimit] = createSignal(
		props.channel.user_limit ?? 0,
	);
	const [editingBitrate, setEditingBitrate] = createSignal(
		props.channel.bitrate ?? 65535,
	);
	const [editingIcon, setEditingIcon] = createSignal(props.channel.icon);
	const [editorState, setEditorState] = createSignal<EditorState | null>(null);

	let iconInputEl!: HTMLInputElement;

	const editor = createEditor({
		channelId: () => props.channel.id,
		roomId: () => props.channel.room_id,
		initialContent: props.channel.description as string | undefined,
	});

	onMount(() => {
		if (editorState()) {
			editor.setState(editorState() as any);
		}
	});

	const isGdm = () => props.channel.type === "Gdm";

	const setIconFile = async (f: File) => {
		await createUpload({
			client: api.client,
			file: f,
			onComplete(media) {
				setEditingIcon(media.id);
				api.client.http.PATCH("/api/v1/channel/{channel_id}", {
					params: { path: { channel_id: props.channel.id } },
					body: { icon: media.id },
				});
			},
			onFail(_error) {},
			onPause() {},
			onResume() {},
			onProgress(_progress) {},
		});
	};

	const removeIcon = async () => {
		setEditingIcon(null);
		await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: props.channel.id } },
			body: { icon: null },
		});
	};

	const openIconPicker = () => {
		iconInputEl?.click();
	};

	const hasVoice = () => {
		const type = channels2.cache.get(props.channel.id)?.type;
		return type === "Voice" || type === "Broadcast";
	};

	const isDirty = () =>
		editingName() !== props.channel.name ||
		getDescriptionFromState() !== props.channel.description ||
		editingNsfw() !== props.channel.nsfw ||
		editingSlowmodeMessage() !== props.channel.slowmode_message ||
		editingSlowmodeThread() !== props.channel.slowmode_thread ||
		editingDefaultSlowmodeMessage() !==
			props.channel.default_slowmode_message ||
		(hasVoice() &&
			(editingUserLimit() !== (props.channel.user_limit ?? 0) ||
				editingBitrate() !== (props.channel.bitrate ?? 65535))) ||
		(isGdm() && editingIcon() !== props.channel.icon);

	const getDescriptionFromState = () => {
		if (!editorState()) return "";
		const text = editorState()!.doc.textContent;
		return text;
	};

	const save = () => {
		const description = getDescriptionFromState();
		ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: props.channel.id } },
			body: {
				name: editingName(),
				description,
				nsfw: editingNsfw(),
				slowmode_message: editingSlowmodeMessage(),
				slowmode_thread: editingSlowmodeThread(),
				default_slowmode_message: editingDefaultSlowmodeMessage(),
				...(hasVoice() && {
					user_limit: editingUserLimit() === 0 ? null : editingUserLimit(),
					bitrate: editingBitrate(),
				}),
				...(isGdm() && { icon: editingIcon() }),
			},
		});
	};

	const toggleArchived = () => {
		if (props.channel.archived_at) {
			channels2.unarchive(props.channel.id);
		} else {
			channels2.archive(props.channel.id);
		}
	};

	const toggleLocked = () => {
		if (props.channel.locked) {
			channels2.unlock(props.channel.id);
		} else {
			channels2.lock(props.channel.id);
		}
	};

	const reset = () => {
		setEditingName(props.channel.name);
		setEditingNsfw(props.channel.nsfw);
		setEditingSlowmodeMessage(props.channel.slowmode_message);
		setEditingSlowmodeThread(props.channel.slowmode_thread);
		setEditingDefaultSlowmodeMessage(props.channel.default_slowmode_message);
		setEditingUserLimit(props.channel.user_limit ?? 0);
		setEditingBitrate(props.channel.bitrate ?? 65535);
		setEditingIcon(props.channel.icon);
		setEditorState(
			(editor as any).createEditorState({
				doc: editor.schema.nodes.doc.create(
					null,
					editor.schema.text(props.channel.description as string),
				),
			}),
		);
	};

	return (
		<div>
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
			<editor.View
				onChange={(state) => setEditorState(state)}
				channelId={props.channel.id}
				submitOnEnter={false}
				autofocus={false}
			/>
			<br />
			<br />
			<Show when={isGdm()}>
				<div>
					<div class="dim">icon</div>
					<div class="avatar-uploader" onClick={openIconPicker}>
						<div class="avatar-inner">
							<ChannelIconGdm id={props.channel.id} icon={editingIcon()} />
							<div class="overlay">upload icon</div>
						</div>
						<Show when={editingIcon()}>
							<button
								class="remove"
								onClick={(e) => {
									e.stopPropagation();
									removeIcon();
								}}
							>
								remove
							</button>
						</Show>
						<input
							style="display:none"
							ref={iconInputEl}
							type="file"
							onInput={(e) => {
								const f = e.target.files?.[0];
								if (f) setIconFile(f);
							}}
						/>
					</div>
				</div>
				<br />
				<br />
			</Show>
			<div>
				channel id: <code class="select-all">{props.channel.id}</code>
			</div>
			<div>
				<div class="dim">slowmode (messages)</div>
				<DurationInput
					value={editingSlowmodeMessage()}
					onInput={(d) =>
						setEditingSlowmodeMessage(typeof d === "number" ? d : null)}
					presets={slowmodePresets}
					placeholder="disabled"
				/>
			</div>
			<div>
				<div class="dim">slowmode (threads)</div>
				<DurationInput
					value={editingSlowmodeThread()}
					onInput={(d) =>
						setEditingSlowmodeThread(typeof d === "number" ? d : null)}
					presets={slowmodePresets}
					placeholder="disabled"
				/>
			</div>
			<Show
				when={channels2.cache.get(props.channel.id)?.type === "Forum" ||
					channels2.cache.get(props.channel.id)?.type === "Text"}
			>
				<div>
					<div class="dim">slowmode (messages default for threads)</div>
					<DurationInput
						value={editingDefaultSlowmodeMessage()}
						onInput={(d) =>
							setEditingDefaultSlowmodeMessage(
								typeof d === "number" ? d : null,
							)}
						presets={slowmodePresets}
						placeholder="disabled"
					/>
				</div>
			</Show>
			<Show when={hasVoice()}>
				<div style="margin-top: 8px">
					<div class="dim">user limit</div>
					<div
						class="slider-container"
						style="display: flex; align-items: center; gap: 8px; margin: 8px 0; margin-top: 0"
					>
						<input
							type="range"
							min="0"
							max="100"
							value={editingUserLimit()}
							onInput={(e) =>
								setEditingUserLimit(Number(e.currentTarget.value))}
							style="flex: 1;"
						/>
						<span style="min-width: 60px; text-align: right;">
							{editingUserLimit() === 0 ? "Unlimited" : editingUserLimit()}
						</span>
					</div>
				</div>
				<div style="margin-top: 8px">
					<div class="dim">bitrate</div>
					<div
						class="slider-container"
						style="display: flex; align-items: center; gap: 8px; margin: 8px 0; margin-top: 0"
					>
						<input
							type="range"
							min="0"
							max="96000"
							step="1000"
							value={editingBitrate()}
							onInput={(e) => setEditingBitrate(Number(e.currentTarget.value))}
							style="flex: 1;"
							list="bitrate-detents"
						/>
						<datalist id="bitrate-detents">
							<option value="64000" label="64k" />
						</datalist>
						<span style="min-width: 60px; text-align: right;">
							{Math.round(editingBitrate() / 1000)}k
						</span>
					</div>
				</div>
			</Show>
			<div>
				<CheckboxOption
					id={`channel-${props.channel.id}-nsfw`}
					checked={editingNsfw()}
					onChange={setEditingNsfw}
					seed={`channel-${props.channel.id}-nsfw`}
				>
					<Checkbox
						checked={editingNsfw()}
						seed={`channel-${props.channel.id}-nsfw`}
					/>
					<div>
						<b>nsfw</b>
						<div>mark this channel as not safe for work</div>
					</div>
				</CheckboxOption>
			</div>
			<Show when={props.channel.type === "Forum"}>
				<div class="tags">
					<h3 class="dim">Tags</h3>
					<div class="tag-list">
						<For each={props.channel.tags_available!}>
							{(tag) => (
								<div
									class="tag-item"
									style={{
										background: tag.color as string | undefined,
										opacity: tag.archived ? 0.6 : 1,
									}}
									onClick={() => {
										modalctl.open({
											type: "tag_editor",
											forumChannelId: props.channel.id,
											tag: tag,
										});
									}}
								>
									<span class="tag-name">{tag.name}</span>
									<span class="tag-count">{tag.active_thread_count}</span>
								</div>
							)}
						</For>
					</div>
					<button
						class="secondary small"
						onClick={() => {
							modalctl.open({
								type: "tag_editor",
								forumChannelId: props.channel.id,
							});
						}}
					>
						Add New Tag
					</button>
				</div>
			</Show>
			{/* TODO: add/remove tags from thread channels */}
			{/* TODO: archive all threads in this channel (text, forum) */}
			<Savebar
				show={isDirty()}
				onCancel={reset}
				onSave={save}
			/>
		</div>
	);
}
