import type { Channel } from "sdk";
import { createSignal, For, Show, type VoidProps } from "solid-js";
import { useCtx } from "../context.ts";
import { useApi } from "../api.tsx";
import { useModals } from "../contexts/modal";

export function Info(props: VoidProps<{ channel: Channel }>) {
	const ctx = useCtx();
	const api = useApi();
	const [, modalctl] = useModals();
	const [editingNsfw, setEditingNsfw] = createSignal(props.channel.nsfw);
	const [editingName, setEditingName] = createSignal(props.channel.name);
	const [editingDescription, setEditingDescription] = createSignal(
		props.channel.description,
	);

	const isDirty = () =>
		editingName() !== props.channel.name ||
		editingDescription() !== props.channel.description ||
		editingNsfw() !== props.channel.nsfw;

	const save = () => {
		ctx.client.http.PATCH("/api/v1/channel/{channel_id}", {
			params: { path: { channel_id: props.channel.id } },
			body: {
				name: editingName(),
				description: editingDescription(),
				nsfw: editingNsfw(),
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
				<label>
					<div>
						<input
							type="checkbox"
							checked={editingNsfw()}
							onInput={(e) => setEditingNsfw(e.currentTarget.checked)}
						/>
						<b>nsfw</b>
					</div>
					<div>mark this channel as not safe for work</div>
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
			<br />
			{/* TODO: add padding to all settings */}
			<div class="danger" style="margin:0 2px">
				<h3>danger zone</h3>
				<label>
					{/* should this really be in the "danger zone"? archiving doesnt do much */}
					<button onClick={toggleArchived}>
						{props.channel.archived_at ? "unarchive" : "archive"}
					</button>
					<span style="margin-left:8px">
						{props.channel.archived_at
							? "shows this channel in the nav bar"
							: "hides this channel in the nav bar"}
					</span>
				</label>
				<br />
				<label>
					<button onClick={toggleLocked}>
						{props.channel.locked ? "unlock" : "lock"}
					</button>
					<span style="margin-left:8px">
						{props.channel.locked
							? "anyone will be able to chat in this channel"
							: "only moderators can chat in this channel"}
					</span>
				</label>
				<br />
				<label>
					<button onClick={() => alert("todo")}>remove</button>
					<span style="margin-left:8px">
						archives and locks this channel and hides it from all listings
						(direct links still work)
					</span>
				</label>
				<br />
			</div>
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
