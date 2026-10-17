import type { EditorState } from "prosemirror-state";
import type { Channel } from "sdk";
import { createUpload } from "sdk";
import { createSignal, onMount, Show, type VoidProps } from "solid-js";
import { createStore } from "solid-js/store";
import { useApi, useChannels } from "@/api";
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

const autoArchivePresets: DurationPreset[] = [
	{ label: "disabled", seconds: null as any },
	{ label: "1 hour", seconds: 3600 },
	{ label: "6 hours", seconds: 21600 },
	{ label: "24 hours", seconds: 86400 },
	{ label: "3 days", seconds: 86400 * 3 },
	{ label: "7 days", seconds: 86400 * 7 },
];

type Draft = {
	name: string;
	nsfw: boolean;
	autoArchive: number | null;
	defaultAutoArchive: number | null;
	slowmodeMessage: number | null;
	slowmodeThread: number | null;
	defaultSlowmodeMessage: number | null;
	userLimit: number;
	bitrate: number;
	icon: string | null;
	url: string;
};

const toDraft = (c: Channel): Draft => ({
	name: c.name,
	nsfw: c.nsfw ?? false,
	autoArchive: c.auto_archive_duration ?? null,
	defaultAutoArchive: c.default_auto_archive_duration ?? null,
	slowmodeMessage: c.slowmode_message ?? null,
	slowmodeThread: c.slowmode_thread ?? null,
	defaultSlowmodeMessage: c.default_slowmode_message ?? null,
	userLimit: c.user_limit ?? 0,
	bitrate: c.bitrate ?? 65535,
	icon: c.icon ?? null,
	url: c.url ?? "",
});

export function Info(props: VoidProps<{ channel: Channel }>) {
	const api = useApi();
	const channels = useChannels();
	const [draft, setDraft] = createStore(toDraft(props.channel));
	const [description, setDescription] = createSignal<string | null>(null);
	const [saving, setSaving] = createSignal(false);

	let iconInputEl!: HTMLInputElement;

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const editor = createEditor({
		channelId: () => props.channel.id ?? "",
		roomId: () => props.channel.room_id ?? "",
		toolbar,
		autocomplete,
		initialContent: () => props.channel.description ?? "",
	});

	const isGdm = () => props.channel.type === "Gdm";
	const isInfo = () => props.channel.type === "Info";

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
		description() !== (props.channel.description ?? "");

	const hasThreadSlowmode = () => {
		return (
			props.channel.type === "Forum" ||
			props.channel.type === "Forum2" ||
			props.channel.type === "Ticket" ||
			props.channel.type === "Announcement" ||
			props.channel.type === "Wiki" ||
			props.channel.type === "Text"
		);
		// TODO: (props.channel.type === "Document" || props.channel.parent.type !== "Wiki")
	};

	const hasTextSlowmode = () => {
		return (
			props.channel.type === "ThreadPublic" ||
			props.channel.type === "ThreadPrivate" ||
			props.channel.type === "ThreadForum2" ||
			props.channel.type === "Voice" ||
			props.channel.type === "Announcement" ||
			props.channel.type === "Text"
		);
	};

	const hasAutoArchive = () => {
		return (
			props.channel.type === "ThreadPublic" ||
			props.channel.type === "ThreadPrivate" ||
			props.channel.type === "ThreadForum2"
		);
	};

	const hasDefaultAutoArchive = () => {
		return (
			props.channel.type === "Forum" ||
			props.channel.type === "Forum2" ||
			props.channel.type === "Ticket" ||
			props.channel.type === "Announcement" ||
			props.channel.type === "Wiki" ||
			props.channel.type === "Text"
		);
		// TODO: (props.channel.type === "Document" || props.channel.parent.type !== "Wiki")
	};

	const save = async () => {
		setSaving(true);
		const updated = await channels.update(props.channel.id, {
			name: draft.name,
			description: description() || null,
			nsfw: draft.nsfw,
			...(hasAutoArchive() && {
				auto_archive_duration: draft.autoArchive,
			}),
			...(hasDefaultAutoArchive() && {
				default_auto_archive_duration: draft.defaultAutoArchive,
			}),
			slowmode_message: draft.slowmodeMessage,
			slowmode_thread: draft.slowmodeThread,
			default_slowmode_message: draft.defaultSlowmodeMessage,
			...(hasVoice() && {
				user_limit: draft.userLimit === 0 ? null : draft.userLimit,
				bitrate: draft.bitrate,
			}),
			...(isGdm() && { icon: draft.icon }),
			...(isInfo() && { url: draft.url || null }),
		});
		setSaving(false);
		// TODO: reset draft once saved
		// TODO: reset draft if its not dirty and channel is updated
		// TODO: disable inputs while saving draft
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
		const s = editor.createState();
		editor.setState(s);
		setDescription(s.doc.textContent);
	};

	return (
		<div class="channel-settings-info">
			<h2>info</h2>
			<br />
			<div class="channel-profile">
				<Show when={isGdm()}>
					<label class="channel-icon-upload">
						<h3 class="label">icon</h3>
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
						<h3 class="label">name</h3>
						<input
							value={draft.name}
							type="text"
							class="name-input"
							onInput={(e) => setDraft("name", e.target.value)}
							placeholder="frog"
						/>
					</label>
					<label class="description">
						<h3 class="label">description</h3>
						<editor.View
							onChange={(state) => setDescription(state.doc.textContent)}
							channelId={props.channel.id}
							submitOnEnter={false}
							autofocus={false}
							placeholder="post pictures of frogs"
						/>
					</label>
				</div>
			</div>
			<Show when={hasDefaultAutoArchive()}>
				<div class="settings-group">
					<label>
						<h3 class="label">Archive threads after</h3>
						<p class="description">
							Automatically archive new threads after being inactive for this
							long.
						</p>
						<DurationInput
							value={draft.defaultAutoArchive}
							onInput={(d) =>
								setDraft("defaultAutoArchive", typeof d === "number" ? d : null)
							}
							presets={autoArchivePresets}
							placeholder="disabled"
						/>
					</label>
				</div>
			</Show>
			<Show when={hasAutoArchive()}>
				<div class="settings-group">
					<label>
						<h3 class="label">Archive after</h3>
						<p class="description">
							Automatically archive this thread after being inactive for this
							long.
						</p>
						<DurationInput
							value={draft.autoArchive}
							onInput={(d) =>
								setDraft("autoArchive", typeof d === "number" ? d : null)
							}
							presets={autoArchivePresets}
							placeholder="disabled"
						/>
					</label>
				</div>
			</Show>
			<Show when={hasTextSlowmode() || hasThreadSlowmode()}>
				<div class="settings-group">
					<h3 class="label">Slowmode</h3>
					<p class="description">
						Limit how quickly chatters can send messages and create threads.
						Members with{" "}
						<span
							style="text-decoration:underline dotted"
							// TODO: localize this
							data-tooltip="ChannelManage, ThreadManage, MemberTimeout, ChannelSlowmodeBypass"
						>
							certain permissions
						</span>{" "}
						aren't affected by this.
					</p>
					<div style="display:flex;gap:8px">
						<Show when={hasTextSlowmode()}>
							<label>
								<h3 class="dim" style="margin:0 2px">
									messages
								</h3>
								<DurationInput
									value={draft.slowmodeMessage}
									onInput={(d) =>
										setDraft(
											"slowmodeMessage",
											typeof d === "number" ? d : null,
										)
									}
									presets={slowmodePresets}
									placeholder="disabled"
								/>
							</label>
						</Show>
						<Show when={hasThreadSlowmode()}>
							<label>
								<h3 class="dim" style="margin:0 2px">
									threads
								</h3>
								<DurationInput
									value={draft.slowmodeThread}
									onInput={(d) =>
										setDraft("slowmodeThread", typeof d === "number" ? d : null)
									}
									presets={slowmodePresets}
									placeholder="disabled"
								/>
							</label>
							<label>
								<h3
									class="dim"
									style="margin:0 2px"
									data-tooltip="This is copied to newly created threads and does not apply retroactively"
								>
									messages in threads
								</h3>
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
					</div>
				</div>
			</Show>
			<Show when={hasVoice()}>
				<div class="settings-group">
					<label>
						<h3 class="label">User limit</h3>
						<p class="description dim">
							How many members can join this voice channel at once.
						</p>
						<div
							class="slider-container"
							style="display: flex; align-items: center; gap: 8px"
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
							<span style="min-width: 80px; text-align: right;">
								{draft.userLimit === 0 ? "Unlimited" : draft.userLimit}
							</span>
						</div>
					</label>
				</div>
				<div class="settings-group">
					<label>
						<h3 class="label">Bitrate</h3>
						<p class="description dim">
							Set the audio quality fo this voice channel.
						</p>
						<div
							class="slider-container"
							style="display: flex; align-items: center; gap: 8px"
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
							<span style="min-width: 80px; text-align: right;">
								{Math.round(draft.bitrate / 1000)}k
							</span>
						</div>
					</label>
				</div>
			</Show>
			<Show when={!isInfo() /* TODO: hasNsfw() */}>
				<div class="settings-group">
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
			</Show>
			<Show when={isInfo()}>
				<div class="settings-group">
					<label style="display:flex;flex-direction:column">
						<h3 class="label">url</h3>
						<p class="description">The link this info channel points to.</p>
						<input
							value={draft.url}
							type="url"
							class="name-input"
							onInput={(e) => setDraft("url", e.target.value)}
						/>
					</label>
				</div>
			</Show>
			{/* TODO: add/remove tags from thread channels */}
			{/* TODO: archive all threads in this channel (text, forum) */}
			<Savebar
				show={isDirty()}
				saving={saving()}
				onCancel={reset}
				onSave={save}
			/>
		</div>
	);
}
