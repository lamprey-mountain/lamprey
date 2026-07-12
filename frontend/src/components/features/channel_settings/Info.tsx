import { createStore } from "solid-js/store";
import type { EditorState } from "prosemirror-state";
import type { Channel } from "sdk";
import { createUpload } from "sdk";
import { createSignal, For, onMount, Show, type VoidProps } from "solid-js";
import { useApi, useChannels } from "@/api";
import { useCtx } from "@/app/context";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { DurationInput, type DurationPreset } from "@/atoms/DurationInput.tsx";
import { Checkbox } from "@/atoms/icons";
import { Savebar } from "@/atoms/Savebar";
import { createEditor } from "@/components/features/editor/Editor.tsx";
import { ChannelIconGdm } from "@/components/shared/User";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";

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

type Draft = {
	name: string;
	nsfw: boolean;
	slowmodeMessage: number | null;
	slowmodeThread: number | null;
	defaultSlowmodeMessage: number | null;
	userLimit: number;
	bitrate: number;
	icon: string | null;
};

const toDraft = (c: Channel): Draft => ({
	name: c.name,
	nsfw: c.nsfw ?? false,
	slowmodeMessage: c.slowmode_message ?? null,
	slowmodeThread: c.slowmode_thread ?? null,
	defaultSlowmodeMessage: c.default_slowmode_message ?? null,
	userLimit: c.user_limit ?? 0,
	bitrate: c.bitrate ?? 65535,
	icon: c.icon ?? null,
});

export function Info(props: VoidProps<{ channel: Channel }>) {
	const ctx = useCtx();
	const api = useApi();
	const channels = useChannels();
	const [draft, setDraft] = createStore(toDraft(props.channel));
	const [editorState, setEditorState] = createSignal<EditorState | null>(null);

	let iconInputEl!: HTMLInputElement;

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const editor = createEditor({
		channelId: () => props.channel.id ?? "",
		roomId: () => props.channel.room_id ?? "",
		toolbar,
		autocomplete,
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
				setDraft("icon", media.id);
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
		setDraft("icon", null);
		await api.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: props.channel.id } },
			body: { icon: null },
		});
	};

	const openIconPicker = () => {
		iconInputEl?.click();
	};

	const hasVoice = () => {
		const type = channels.cache.get(props.channel.id)?.type;
		return type === "Voice" || type === "Broadcast";
	};

	const isDirty = () =>
		JSON.stringify(draft) !== JSON.stringify(toDraft(props.channel)) ||
		getDescriptionFromState() !== (props.channel.description ?? "");

	const getDescriptionFromState = () => {
		if (!editorState()) return "";
		const text = editorState()?.doc.textContent;
		return text;
	};

	const save = () => {
		const description = getDescriptionFromState();
		ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: props.channel.id } },
			body: {
				name: draft.name,
				description,
				nsfw: draft.nsfw,
				slowmode_message: draft.slowmodeMessage,
				slowmode_thread: draft.slowmodeThread,
				default_slowmode_message: draft.defaultSlowmodeMessage,
				...(hasVoice() && {
					user_limit: draft.userLimit === 0 ? null : draft.userLimit,
					bitrate: draft.bitrate,
				}),
				...(isGdm() && { icon: draft.icon }),
			},
		});
	};

	const _toggleArchived = () => {
		if (props.channel.archived_at) {
			channels.unarchive(props.channel.id);
		} else {
			channels.archive(props.channel.id);
		}
	};

	const _toggleLocked = () => {
		if (props.channel.locked) {
			channels.unlock(props.channel.id);
		} else {
			channels.lock(props.channel.id);
		}
	};

	const reset = () => {
		setDraft(toDraft(props.channel));
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
		<div class="channel-settings-info">
			<h2>info</h2>
			<div class="channel-profile">
				<Show when={isGdm()}>
					<label class="channel-icon">
						<h3 class="dim">icon</h3>
						<div class="avatar-uploader" onClick={openIconPicker}>
							<div class="avatar-inner">
								<ChannelIconGdm id={props.channel.id} icon={draft.icon} />
								<div class="overlay">upload icon</div>
							</div>
							<Show when={draft.icon}>
								{/* TODO: keyboard a11y (tabindex, style, onKeydown/press)*/}
								<button
									type="button"
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
					</label>
				</Show>
				<div class="name-description">
					<label class="name">
						<h3 class="dim">name</h3>
						<input
							value={draft.name}
							type="text"
							class="name-input"
							onInput={(e) => setDraft("name", e.target.value)}
						/>
					</label>
					<label class="description">
						<h3 class="dim">description</h3>
						<editor.View
							onChange={(state) => setEditorState(state)}
							channelId={props.channel.id}
							submitOnEnter={false}
							autofocus={false}
						/>
					</label>
				</div>
			</div>
			<label>
				<h3 class="dim">slowmode (messages)</h3>
				<DurationInput
					value={draft.slowmodeMessage}
					onInput={(d) =>
						setDraft("slowmodeMessage", typeof d === "number" ? d : null)
					}
					presets={slowmodePresets}
					placeholder="disabled"
				/>
			</label>
			<label>
				<h3 class="dim">slowmode (threads)</h3>
				<DurationInput
					value={draft.slowmodeThread}
					onInput={(d) =>
						setDraft("slowmodeThread", typeof d === "number" ? d : null)
					}
					presets={slowmodePresets}
					placeholder="disabled"
				/>
			</label>
			<Show
				when={
					channels.cache.get(props.channel.id)?.type === "Forum" ||
					channels.cache.get(props.channel.id)?.type === "Text"
				}
			>
				<label>
					<h3 class="dim">slowmode (messages default for threads)</h3>
					<DurationInput
						value={draft.defaultSlowmodeMessage}
						onInput={(d) =>
							setDraft(
								"defaultSlowmodeMessage",
								typeof d === "number" ? d : null,
							)
						}
						presets={slowmodePresets}
						placeholder="disabled"
					/>
				</label>
			</Show>
			<Show when={hasVoice()}>
				<label style="margin-top: 8px; display: block">
					<h3 class="dim">user limit</h3>
					<div
						class="slider-container"
						style="display: flex; align-items: center; gap: 8px; margin: 8px 0; margin-top: 0"
					>
						<input
							type="range"
							min="0"
							max="100"
							value={draft.userLimit}
							onInput={(e) =>
								setDraft("userLimit", Number(e.currentTarget.value))
							}
							style="flex: 1;"
						/>
						<span style="min-width: 60px; text-align: right;">
							{draft.userLimit === 0 ? "Unlimited" : draft.userLimit}
						</span>
					</div>
				</label>
				<label style="margin-top: 8px; display: block">
					<h3 class="dim">bitrate</h3>
					<div
						class="slider-container"
						style="display: flex; align-items: center; gap: 8px; margin: 8px 0; margin-top: 0"
					>
						<input
							type="range"
							min="0"
							max="96000"
							step="1000"
							value={draft.bitrate}
							onInput={(e) =>
								setDraft("bitrate", Number(e.currentTarget.value))
							}
							style="flex: 1;"
							list="bitrate-detents"
						/>
						<datalist id="bitrate-detents">
							<option value="64000" label="64k" />
						</datalist>
						<span style="min-width: 60px; text-align: right;">
							{Math.round(draft.bitrate / 1000)}k
						</span>
					</div>
				</label>
			</Show>
			<div>
				<CheckboxOption
					id={`channel-${props.channel.id}-nsfw`}
					checked={draft.nsfw ?? false}
					onChange={(v) => setDraft("nsfw", v)}
					seed={`channel-${props.channel.id}-nsfw`}
				>
					<Checkbox
						checked={draft.nsfw ?? false}
						seed={`channel-${props.channel.id}-nsfw`}
					/>
					<div>
						<b>nsfw</b>
						<div>mark this channel as not safe for work</div>
					</div>
				</CheckboxOption>
			</div>
			{/* TODO: add/remove tags from thread channels */}
			{/* TODO: archive all threads in this channel (text, forum) */}
			<Savebar show={isDirty()} onCancel={reset} onSave={save} />
		</div>
	);
}
