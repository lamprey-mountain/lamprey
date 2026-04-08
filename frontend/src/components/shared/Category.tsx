import { A, useNavigate } from "@solidjs/router";
import type { EditorState } from "prosemirror-state";
import { type Channel, getTimestampFromUUID } from "sdk";
import { createMemo, createSignal, For, Show } from "solid-js";
import { uuidv7 } from "uuidv7";
import { useChannels } from "@/api";
import { Time } from "@/atoms/Time";
import { RenderUploadItem } from "@/components/features/chat/Input.tsx";
import { createEditor } from "@/components/features/editor/Editor.tsx";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useModals } from "@/contexts/modal";
import { useUploads } from "@/contexts/uploads.tsx";
import { useMessageSubmit } from "@/hooks/useMessageSubmit.ts";
import { usePermissions } from "@/hooks/usePermissions";
import { flags } from "@/lib/flags";
import { md } from "@/lib/markdown";

export const Category = (props: { channel: Channel }) => {
	const channels2 = useChannels();
	const nav = useNavigate();
	const [, modalCtl] = useModals();
	const room_id = () => props.channel.room_id ?? "";

	const [threadFilter, setThreadFilter] = createSignal("active");

	const threadsResource = createMemo(() => {
		const rid = room_id();
		if (!rid) return [];
		return [...channels2.cache.values()].filter((c) => c.room_id === rid);
	});

	const getThreads = () => {
		const items = threadsResource();
		if (!items) return [];
		// sort descending by id
		return [...items]
			.filter((t) => t.parent_id === props.channel.id)
			.sort((a, b) => (a.id < b.id ? 1 : -1));
	};

	function createThread(room_id: string) {
		modalCtl.prompt("name?", (name) => {
			if (!name) return;
			channels2.create(room_id, { name });
		});
	}

	const u = useCurrentUser();
	const user_id = () => u()?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	return (
		<div class="room-home">
			<div style="display:flex">
				<div style="flex:1">
					<h2>{props.channel.name}</h2>
					<p
						class="markdown"
						innerHTML={md(props.channel.description ?? "") as string}
					></p>
				</div>
				<div style="display:flex;flex-direction:column;gap:4px">
					<A
						style="padding: 0 4px"
						href={`/channel/${props.channel.id}/settings`}
					>
						settings
					</A>
				</div>
			</div>
			<Show when={flags.has("thread_quick_create")}>
				<br />
				<QuickCreate channel={props.channel} />
				<br />
			</Show>
			<div style="display:flex; align-items:center">
				<h3 style="font-size:1rem; margin-top:8px;flex:1">
					{getThreads().length} {threadFilter()} threads
				</h3>
				<div class="thread-filter">
					<button
						type="button"
						class="button"
						classList={{ selected: threadFilter() === "active" }}
						onClick={[setThreadFilter, "active"]}
					>
						active
					</button>
					<button
						type="button"
						class="button"
						classList={{ selected: threadFilter() === "archived" }}
						onClick={[setThreadFilter, "archived"]}
					>
						archived
					</button>
					<Show when={perms.has("ThreadManage")}>
						<button
							type="button"
							class="button"
							classList={{ selected: threadFilter() === "removed" }}
							onClick={[setThreadFilter, "removed"]}
						>
							removed
						</button>
					</Show>
				</div>
				<button
					type="button"
					class="primary"
					style="margin-left: 8px;border-radius:4px"
					onClick={() => {
						const rid = room_id();
						if (rid) createThread(rid);
					}}
				>
					create thread
				</button>
			</div>
			<ul>
				<For each={getThreads()}>
					{(thread) => (
						<li>
							<article class="thread menu-thread" data-thread-id={thread.id}>
								<button
									type="button"
									class="top"
									onClick={() => nav(`/thread/${thread.id}`)}
									onKeyDown={(e) =>
										e.key === "Enter" && nav(`/thread/${thread.id}`)
									}
								>
									<div class="icon"></div>
									<div class="spacer">{thread.name}</div>
									<div class="time">
										Created <Time date={getTimestampFromUUID(thread.id)} />
									</div>
								</button>
								<button
									type="button"
									class="bottom"
									onClick={() => nav(`/thread/${thread.id}`)}
									onKeyDown={(e) =>
										e.key === "Enter" && nav(`/thread/${thread.id}`)
									}
								>
									<div class="dim">
										{thread.message_count} message(s) &bull; last msg{" "}
										<Time
											date={getTimestampFromUUID(
												thread.last_version_id ?? thread.id,
											)}
										/>
									</div>
									<Show when={thread.description}>
										<div
											class="description markdown"
											innerHTML={md(thread.description ?? "") as string}
										></div>
									</Show>
								</button>
							</article>
						</li>
					)}
				</For>
			</ul>
		</div>
	);
};

// NOTE the room id is reused as the channel id for draft messages and attachments
const QuickCreate = (props: { channel: Channel }) => {
	const channels2 = useChannels();
	const n = useNavigate();
	const channelCtx = useChannel();
	const submit = useMessageSubmit(props.channel.id);
	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	return (
		<Show when={channelCtx} fallback={<div>Loading editor...</div>}>
			{(ctx) => {
				const [ch, chUpdate] = ctx();
				const editor = createEditor({
					channelId: () => props.channel.id,
					roomId: () => props.channel.room_id ?? "",
					toolbar,
					autocomplete,
				});

				function uploadFile(e: InputEvent) {
					const target = e.target as HTMLInputElement;
					if (!target.files) return;
					const files = Array.from(target.files);
					for (const file of files) {
						handleUpload(file);
					}
				}

				const uploads = useUploads();

				function handleUpload(file: File) {
					console.log(file);
					const local_id = uuidv7();
					uploads.init(local_id, props.channel.id, file);
				}

				const onSubmit = (text: string) => {
					if (!text) return false;
					const room_id = props.channel.room_id;
					if (!room_id) return false;
					channels2
						.create(room_id, {
							name: "thread",
							parent_id: props.channel.id,
						})
						.then((t) => {
							if (!t) return;
							submit(text, false, t.id);
							n(`/channel/${t.id}`);
						});
					return true;
				};

				const onChange = (state: EditorState) => {
					chUpdate("editor_state", state);
				};

				const atts = () => ch.attachments;
				return (
					<div class="message-input quick-create">
						<div style="margin-bottom: 2px">quick create thread</div>
						<Show when={atts()?.length}>
							<div class="attachments">
								<header>
									{atts()?.length}{" "}
									{atts()?.length === 1 ? "attachment" : "attachments"}
								</header>
								<ul>
									<For each={atts()}>
										{(att) => (
											<RenderUploadItem
												thread_id={props.channel.id}
												att={att}
											/>
										)}
									</For>
								</ul>
							</div>
						</Show>
						<div class="text">
							<label class="upload">
								+
								<input
									multiple
									type="file"
									onInput={uploadFile}
									value="upload file"
								/>
							</label>
							<editor.View
								onSubmit={onSubmit}
								onChange={onChange}
								onUpload={handleUpload}
								placeholder={"send a message..."}
							/>
						</div>
					</div>
				);
			}}
		</Show>
	);
};
