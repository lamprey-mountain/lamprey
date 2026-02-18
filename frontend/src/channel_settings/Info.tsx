import type { Channel } from "sdk";
import { createSignal, For, Show, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";
import { useApi } from "../api.tsx";
import { useModals } from "../contexts/modal";
import { Checkbox } from "../icons";
import { DurationInput } from "../DurationInput.tsx";

export function Info(props: VoidProps<{ channel: Channel }>) {
	const ctx = useCtx();
	const api = useApi();
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

	const hasVoice = () => {
		const type = api.channels.cache.get(props.channel.id)?.type;
		return type === "Voice" || type === "Broadcast";
	};

	const isDirty = () =>
		editingName() !== props.channel.name ||
		editingDescription() !== props.channel.description ||
		editingNsfw() !== props.channel.nsfw ||
		editingSlowmodeMessage() !== props.channel.slowmode_message ||
		editingSlowmodeThread() !== props.channel.slowmode_thread ||
		editingDefaultSlowmodeMessage() !==
			props.channel.default_slowmode_message ||
		(hasVoice() &&
			(editingUserLimit() !== (props.channel.user_limit ?? 0) ||
				editingBitrate() !== (props.channel.bitrate ?? 65535)));

	const save = () => {
		ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: props.channel.id } },
			body: {
				name: editingName(),
				description: editingDescription(),
				nsfw: editingNsfw(),
				slowmode_message: editingSlowmodeMessage(),
				slowmode_thread: editingSlowmodeThread(),
				default_slowmode_message: editingDefaultSlowmodeMessage(),
				...(hasVoice() && {
					user_limit: editingUserLimit() === 0 ? null : editingUserLimit(),
					bitrate: editingBitrate(),
				}),
			},
		});
	};

	const toggleArchived = () => {
		if (props.channel.archived_at) {
			api.channels.unarchive(props.channel.id);
		} else {
			api.channels.archive(props.channel.id);
		}
	};

	const toggleLocked = () => {
		if (props.channel.locked) {
			api.channels.unlock(props.channel.id);
		} else {
			api.channels.lock(props.channel.id);
		}
	};

	const reset = () => {
		setEditingName(props.channel.name);
		setEditingDescription(props.channel.description);
		setEditingNsfw(props.channel.nsfw);
		setEditingSlowmodeMessage(props.channel.slowmode_message);
		setEditingSlowmodeThread(props.channel.slowmode_thread);
		setEditingDefaultSlowmodeMessage(props.channel.default_slowmode_message);
		setEditingUserLimit(props.channel.user_limit ?? 0);
		setEditingBitrate(props.channel.bitrate ?? 65535);
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
			<textarea
				value={editingDescription()}
				onInput={(e) => setEditingDescription(e.target.value)}
			/>
			<br />
			<br />
			<div>
				channel id: <code class="select-all">{props.channel.id}</code>
			</div>
			<div>
				<div class="dim">slowmode (messages)</div>
				<DurationInput
					value={editingSlowmodeMessage()}
					onInput={setEditingSlowmodeMessage}
				/>
			</div>
			<div>
				<div class="dim">slowmode (threads)</div>
				<DurationInput
					value={editingSlowmodeThread()}
					onInput={setEditingSlowmodeThread}
				/>
			</div>
			<Show
				when={api.channels.cache.get(props.channel.id)?.type === "Forum" ||
					api.channels.cache.get(props.channel.id)?.type === "Text"}
			>
				<div>
					<div class="dim">slowmode (messages default for threads)</div>
					<DurationInput
						value={editingDefaultSlowmodeMessage()}
						onInput={setEditingDefaultSlowmodeMessage}
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
				<label class="option">
					<input
						type="checkbox"
						checked={editingNsfw()}
						onInput={(e) => setEditingNsfw(e.currentTarget.checked)}
						style="display: none;"
					/>
					<Checkbox checked={editingNsfw()} />
					<div>
						<b>nsfw</b>
						<div>mark this channel as not safe for work</div>
					</div>
				</label>
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
										background: tag.color,
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
		</div>
	);
}
