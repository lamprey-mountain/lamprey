import { createMemo, createSignal, For, Show } from "solid-js";
import { useCtx } from "./context.ts";
import { Channel, getTimestampFromUUID } from "sdk";
import { A, useNavigate } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { createEditor } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";
import { EditorState } from "prosemirror-state";
import { RenderUploadItem } from "./Input.tsx";
import { handleSubmit } from "./dispatch/submit.ts";
import { Time } from "./Time.tsx";
import { flags } from "./flags.ts";
import { usePermissions } from "./hooks/usePermissions.ts";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { useChannel } from "./channelctx.tsx";

export const Category = (props: { channel: Channel }) => {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const room_id = () => props.channel.room_id!;

	const [threadFilter, setThreadFilter] = createSignal("active");

	const fetchMore = () => {
		// const filter = threadFilter();
		return api.channels.list(room_id);
		// if (filter === "active") {
		// 	return api.threads.list(room_id);
		// } else if (filter === "archived") {
		// 	return api.threads.listArchived(room_id);
		// } else if (filter === "removed") {
		// 	return api.threads.listRemoved(room_id);
		// }
	};

	const threadsResource = createMemo(fetchMore);

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(() => bottom() ? [bottom()!] : [], (entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) fetchMore();
		}
	});

	const getThreads = () => {
		const items = threadsResource()?.()?.items;
		if (!items) return [];
		// sort descending by id
		return [...items].filter((t) => t.parent_id === props.channel.id).sort((
			a,
			b,
		) => (a.id < b.id ? 1 : -1));
	};

	function createThread(room_id: string) {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				api.channels.create(room_id, { name });
			},
		});
	}

	const user_id = () => api.users.cache.get("@self")?.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	return (
		<div class="room-home">
			<div style="display:flex">
				<div style="flex:1">
					<h2>{props.channel.name}</h2>
					<p>{props.channel.description}</p>
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
						classList={{ selected: threadFilter() === "active" }}
						onClick={[setThreadFilter, "active"]}
					>
						active
					</button>
					<button
						classList={{ selected: threadFilter() === "archived" }}
						onClick={[setThreadFilter, "archived"]}
					>
						archived
					</button>
					<Show when={perms.has("ThreadRemove")}>
						<button
							classList={{ selected: threadFilter() === "removed" }}
							onClick={[setThreadFilter, "removed"]}
						>
							removed
						</button>
					</Show>
				</div>
				<button
					class="primary"
					style="margin-left: 8px;border-radius:4px"
					onClick={() => createThread(room_id())}
				>
					create thread
				</button>
			</div>
			<ul>
				<For each={getThreads()}>
					{(thread) => (
						<li>
							<article class="thread menu-thread" data-thread-id={thread.id}>
								<header onClick={() => nav(`/thread/${thread.id}`)}>
									<div class="top">
										<div class="icon"></div>
										<div class="spacer">{thread.name}</div>
										<div class="time">
											Created <Time date={getTimestampFromUUID(thread.id)} />
										</div>
									</div>
									<div
										class="bottom"
										onClick={() => nav(`/thread/${thread.id}`)}
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
											<div class="description">
												{thread.description}
											</div>
										</Show>
									</div>
								</header>
							</article>
						</li>
					)}
				</For>
			</ul>
			<div ref={setBottom}></div>
		</div>
	);
};

// NOTE the room id is reused as the channel id for draft messages and attachments
const QuickCreate = (
	props: { channel: Channel },
) => {
	const ctx = useCtx();
	const api = useApi();
	const n = useNavigate();
	const [ch, chUpdate] = useChannel()!;

	const editor = createEditor({});

	function uploadFile(e: InputEvent) {
		const target = e.target! as HTMLInputElement;
		const files = Array.from(target.files!);
		for (const file of files) {
			handleUpload(file);
		}
	}

	function handleUpload(file: File) {
		console.log(file);
		const local_id = uuidv7();
		ctx.dispatch({
			do: "upload.init",
			file,
			local_id,
			channel_id: props.channel.id,
		});
	}

	const onSubmit = async (text: string) => {
		if (!text) return;
		const t = await api.channels.create(props.channel.room_id!, {
			name: "thread",
			parent_id: props.channel.id,
		});

		if (!t.data) return;
		handleSubmit(ctx, t.data.id, text, null as any, api, props.channel.id);
		n(`/channel/${t.data.id}`);
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
									channel_id={props.channel.id}
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
};
