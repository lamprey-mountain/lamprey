import { createMemo, createSignal, For, Show } from "solid-js";
import type { RoomT } from "./types.ts";
import { useCtx } from "./context.ts";
import { getTimestampFromUUID } from "sdk";
import { A, useNavigate } from "@solidjs/router";
import { useApi } from "./api.tsx";
import { AvatarWithStatus, UserView } from "./User.tsx";
import { tooltip } from "./Tooltip.tsx";
import { createEditor } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";
import { EditorState } from "prosemirror-state";
import { RenderUploadItem } from "./Input.tsx";
import { handleSubmit } from "./dispatch/submit.ts";
import { Time } from "./Time.tsx";
import { flags } from "./flags.ts";
import { usePermissions } from "./hooks/usePermissions.ts";

export const RoomMembers = (props: { room: RoomT }) => {
	const api = useApi();
	const room_id = () => props.room.id;
	const members = api.room_members.list(room_id);

	return (
		<ul class="member-list" data-room-id={props.room.id}>
			<For
				each={members()?.items.filter((m) => m.membership === "Join")}
				fallback={
					<div class="dim" style="text-align: center; margin-top: 8px">
						no members!
					</div>
				}
			>
				{(member) => {
					const user_id = () => member.user_id;
					const user = api.users.fetch(user_id);

					function name() {
						let name: string | undefined | null = null;
						if (member?.membership === "Join") name ??= member.override_name;

						name ??= user()?.name;
						return name;
					}

					return tooltip(
						{
							placement: "left-start",
						},
						<Show when={user()}>
							<UserView
								user={user()!}
								room_member={member}
							/>
						</Show>,
						<li class="menu-user" data-user-id={member.user_id}>
							<AvatarWithStatus user={user()} />
							<span class="text">
								<span class="name">{name()}</span>
								<Show when={false}>
									<span class="status-message">asdf</span>
								</Show>
							</span>
						</li>,
					);
				}}
			</For>
		</ul>
	);
};

export const RoomHome = (props: { room: RoomT }) => {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const room_id = () => props.room.id;

	const getThreads = createMemo(() => {
		const threads = [...api.threads.cache.values()]
			.filter((t) => t.room_id === props.room.id && !t.deleted_at);
		threads.sort((a, b) => a.id < b.id ? 1 : -1);
		return threads;
	});

	function createThread(room_id: string) {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.POST("/api/v1/room/{room_id}/thread", {
					params: {
						path: { room_id },
					},
					body: { name },
				});
			},
		});
	}

	function leaveRoom(_room_id: string) {
		ctx.dispatch({
			do: "modal.confirm",
			text: "are you sure you want to leave?",
			cont(confirmed) {
				if (!confirmed) return;
				ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id: props.room.id,
							user_id: api.users.cache.get("@self")!.id,
						},
					},
				});
			},
		});
	}

	const [threadFilter, setThreadFilter] = createSignal("active");

	const user_id = () => api.users.cache.get("@self")!.id;
	const perms = usePermissions(user_id, room_id, () => undefined);

	return (
		<div class="room-home">
			<div style="display:flex">
				<div style="flex:1">
					<h2>{props.room.name}</h2>
					<p>{props.room.description}</p>
				</div>
				<div style="display:flex;flex-direction:column;gap:4px">
					<button onClick={() => leaveRoom(room_id())}>leave room</button>
					<A style="padding: 0 4px" href={`/room/${props.room.id}/settings`}>
						settings
					</A>
				</div>
			</div>
			<Show when={flags.has("thread_quick_create")}>
				<br />
				<QuickCreate room={props.room} />
				<br />
			</Show>
			<div style="display:flex; align-items:center">
				<h3 style="font-size:1rem; margin-top:8px;flex:1">
					{getThreads().length} active threads
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
					<Show when={perms.has("ThreadDelete")}>
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
		</div>
	);
};

// NOTE the room id is reused as the thread id for draft messages and attachments
const QuickCreate = (
	props: { room: RoomT },
) => {
	const ctx = useCtx();
	const api = useApi();
	const n = useNavigate();

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
			thread_id: props.room.id,
		});
	}

	const onSubmit = async (text: string) => {
		if (!text) return;
		const t = await ctx.client.http.POST(
			"/api/v1/room/{room_id}/thread",
			{
				params: {
					path: { room_id: props.room.id },
				},
				body: { name: "thread" },
			},
		);

		if (!t.data) return;
		handleSubmit(ctx, t.data.id, text, null as any, api, props.room.id);
		n(`/thread/${t.data.id}`);
	};

	const onChange = (state: EditorState) => {
		// reuse room id as the thread id for draft messages
		ctx.thread_editor_state.set(props.room.id, state);
	};

	const atts = () => ctx.thread_attachments.get(props.room.id);
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
									thread_id={props.room.id}
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
