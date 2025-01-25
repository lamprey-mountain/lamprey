import { produce, reconcile, SetStoreFunction } from "solid-js/store";
import { Action, Data, Slice, TimelineItem } from "./context.ts";
import {
	InviteT,
	MemberT,
	MessageT,
	MessageType,
	Pagination,
	RoleT,
} from "./types.ts";
import { batch as solidBatch } from "solid-js";
import { ChatCtx } from "./context.ts";
import { createEditorState } from "./Editor.tsx";
import { uuidv7 } from "uuidv7";
import { TimelineItemT } from "./Messages.tsx";
import { createUpload } from "sdk";

type RenderTimelineParams = {
	items: Array<TimelineItem>;
	slice: Slice;
	read_marker_id: string | null;
	has_before: boolean;
	has_after: boolean;
};

function renderTimeline(
	{ items, slice, read_marker_id, has_before, has_after }: RenderTimelineParams,
): Array<TimelineItemT> {
	const rawItems = items.slice(slice.start, slice.end) ?? [];
	const newItems: Array<TimelineItemT> = [];
	console.log("renderTimeline", {
		items,
		slice,
		rawItems,
	});

	if (rawItems.length === 0) throw new Error("no items");

	if (has_before) {
		newItems.push({
			type: "info",
			id: "info",
			header: false,
		});
		newItems.push({
			type: "spacer",
			id: "spacer-top",
		});
	} else {
		newItems.push({
			type: "spacer-mini2",
			id: "spacer-top2",
		});
		newItems.push({
			type: "info",
			id: "info",
			header: true,
		});
	}

	for (let i = 0; i < rawItems.length; i++) {
		const msg = rawItems[i];
		if (msg.type === "hole") continue;
		newItems.push({
			type: "message",
			id: msg.message.nonce ?? msg.message.version_id,
			message: msg.message,
			separate: true,
			is_local: msg.type === "local",
			// separate: shouldSplit(messages[i], messages[i - 1]),
		});
		// if (msg.id - prev.originTs > 1000 * 60 * 5) return true;
		// items.push({
		//   type: "message",
		//   id: messages[i].id,
		//   message: messages[i],
		//   separate: true,
		//   // separate: shouldSplit(messages[i], messages[i - 1]),
		// });
		if (msg.message.id === read_marker_id && i !== rawItems.length - 1) {
			newItems.push({
				type: "unread-marker",
				id: "unread-marker",
			});
		}
	}

	if (has_after) {
		newItems.push({
			type: "spacer",
			id: "spacer-bottom",
		});
	} else {
		newItems.push({
			type: "spacer-mini",
			id: "spacer-bottom-mini",
		});
	}

	return newItems;
}

function calculateSlice(
	old: Slice | undefined,
	off: number,
	len: number,
	dir: "b" | "f",
): Slice {
	// messages are approx. 32 px high, show 3 pages of messages
	const SLICE_LEN = Math.ceil(globalThis.innerHeight / 32) * 3;

	// scroll a page at a time
	const PAGINATE_LEN = Math.ceil(globalThis.innerHeight / 32);

	console.log({ old, off, len, dir });

	if (!old) {
		const end = len;
		const start = Math.max(end - SLICE_LEN, 0);
		return { start, end };
	} else if (dir == "b") {
		const start = Math.max(old.start + off - PAGINATE_LEN, 0);
		const end = Math.min(start + SLICE_LEN, len);
		return { start, end };
	} else {
		const end = Math.min(old.end + off + PAGINATE_LEN, len);
		const start = Math.max(end - SLICE_LEN, 0);
		return { start, end };
	}
}

// TODO: refactor this out into multiple smaller files
export function createDispatcher(ctx: ChatCtx, update: SetStoreFunction<Data>) {
	let ackGraceTimeout: number | undefined;
	let ackDebounceTimeout: number | undefined;

	async function fetchMessages(
		thread_id: string,
		from: string,
		dir: "b" | "f",
	) {
		const { data, error } = await ctx.client.http.GET(
			"/api/v1/thread/{thread_id}/message",
			{
				params: {
					path: { thread_id },
					query: {
						dir,
						from,
						limit: 100,
					},
				},
			},
		);
		if (error) throw error;
		return data;
	}

	async function dispatch(action: Action) {
		// console.log("dispatch", action.do);
		console.log("dispatch", action);
		switch (action.do) {
			case "paginate": {
				const { dir, thread_id } = action;
				const oldSlice = ctx.data.slices[thread_id] as Slice | undefined;
				console.log("paginate", { dir, thread_id, oldSlice });

				// fetch items
				let offset: number = 0;
				if (!oldSlice) {
					const from = "ffffffff-ffff-ffff-ffff-ffffffffffff";
					const batch = await fetchMessages(thread_id, from, dir);
					const tl: Array<TimelineItem> = batch.items.map((i: MessageT) => ({
						type: "remote" as const,
						message: i,
					}));
					if (batch.has_more) tl.unshift({ type: "hole" });
					solidBatch(() => {
						update("timelines", thread_id, tl);
						update("slices", thread_id, { start: 0, end: tl.length });
						for (const msg of batch.items) {
							update("messages", msg.id, msg);
						}
						offset = batch.items.length;
						// ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
					});
				} else {
					const tl = ctx.data.timelines[thread_id];
					// console.log({ tl, slice })
					if (tl.length < 2) return; // needs startitem and nextitem
					if (dir === "b") {
						const startItem = tl[oldSlice.start];
						const nextItem = tl[oldSlice.start + 1];
						let batch: Pagination<MessageT> | undefined;
						if (startItem?.type === "hole") {
							const from = nextItem.type === "remote"
								? nextItem.message.id
								: "ffffffff-ffff-ffff-ffff-ffffffffffff";
							batch = await fetchMessages(thread_id, from, dir);
						}
						solidBatch(() => {
							if (batch) {
								update("timelines", thread_id, (i) =>
									[
										...batch.has_more ? [{ type: "hole" }] : [],
										...batch.items.map((j: MessageT) => ({
											type: "remote",
											message: j,
										})),
										...i.slice(oldSlice.start + 1),
									] as Array<TimelineItem>);
								for (const msg of batch.items) {
									update("messages", msg.id, msg);
								}
								offset = batch.items.length;
							}
						});
					} else {
						console.log(oldSlice.start, oldSlice.end, [...tl]);
						const startItem = tl[oldSlice.end - 1];
						const nextItem = tl[oldSlice.end - 2];
						let batch: Pagination<MessageT> | undefined;
						if (startItem.type === "hole") {
							const from = nextItem.type === "remote"
								? nextItem.message.id
								: "00000000-0000-0000-0000-000000000000";
							batch = await fetchMessages(thread_id, from, dir);
						}

						// PERF: indexOf 115ms
						// PERF: reanchor 95.1ms
						// PERF: getting stuff from store? 362ms
						// PERF: setstore: 808ms
						// PERF: set scroll position: 76.6ms
						solidBatch(() => {
							if (batch) {
								update("timelines", thread_id, (i) =>
									[
										...i.slice(0, oldSlice.end - 1),
										...batch.items.map((j: MessageT) => ({
											type: "remote",
											message: j,
										})),
										...batch.has_more ? [{ type: "hole" }] : [],
									] as Array<TimelineItem>);
								for (const msg of batch.items) {
									update("messages", msg.id, msg);
								}

								offset = batch.items.length;
							}
						});
					}
				}

				solidBatch(() => {
					const tl = ctx.data.timelines[thread_id];
					const slice = calculateSlice(oldSlice, offset, tl.length, dir);
					update("slices", thread_id, slice);

					const { read_marker_id } = ctx.data.thread_state[thread_id];
					const newItems = renderTimeline({
						items: tl,
						slice,
						read_marker_id,
						has_before: tl.at(0)?.type === "hole",
						has_after: tl.at(-1)?.type === "hole",
					});
					update(
						"thread_state",
						thread_id,
						"timeline",
						(old) => [...reconcile(newItems)(old)],
					);
				});

				return;
			}
			case "menu": {
				if (action.menu) console.log("handle menu", action.menu);
				update("menu", action.menu);
				return;
			}
			// case "modal.open": {
			// 	updateData("modals", i => [action.modal, ...i ?? []]);
			// 	return;
			// }
			case "modal.close": {
				update("modals", (i) => i.slice(1));
				return;
			}
			case "modal.alert": {
				update(
					"modals",
					(i) => [{ type: "alert", text: action.text }, ...i ?? []],
				);
				return;
			}
			case "modal.confirm": {
				const modal = {
					type: "confirm" as const,
					text: action.text,
					cont: action.cont,
				};
				update("modals", (i) => [modal, ...i]);
				return;
			}
			case "modal.prompt": {
				const modal = {
					type: "prompt" as const,
					text: action.text,
					cont: action.cont,
				};
				update("modals", (i) => [modal, ...i]);
				return;
			}
			case "thread.init": {
				if (ctx.data.thread_state[action.thread_id]) return;
				update("thread_state", action.thread_id, {
					editor_state: createEditorState((text) =>
						handleSubmit(ctx, action.thread_id, text, update)
					),
					reply_id: null,
					scroll_pos: null,
					read_marker_id: action.read_id ?? null,
					attachments: [],
					is_at_end: true,
					timeline: [],
				});
				return;
			}
			case "thread.reply": {
				update("thread_state", action.thread_id, "reply_id", action.reply_id);
				return;
			}
			case "thread.scroll_pos": {
				update("thread_state", action.thread_id, "scroll_pos", action.pos);
				update("thread_state", action.thread_id, "is_at_end", action.is_at_end);
				return;
			}
			case "thread.autoscroll": {
				const { thread_id } = action;
				const ts = ctx.data.thread_state[thread_id];
				console.log(ts);
				if (!ts?.is_at_end) return;

				solidBatch(() => {
					const tl = ctx.data.timelines[thread_id];
					const oldSlice = ctx.data.slices[thread_id];
					const slice = calculateSlice(oldSlice, 1, tl.length, "f");
					update("slices", thread_id, slice);

					const { read_marker_id } = ctx.data.thread_state[thread_id];
					const newItems = renderTimeline({
						items: tl,
						slice,
						read_marker_id,
						has_before: tl.at(0)?.type === "hole",
						has_after: tl.at(-1)?.type === "hole",
					});
					update(
						"thread_state",
						thread_id,
						"timeline",
						(old) => [...reconcile(newItems)(old)],
					);

					const isAtTimelineEnd = tl?.at(-1)?.type !== "hole" &&
						ctx.data.slices[thread_id].end >= tl.length;
					// HACK: solidjs doesn't like me doing this
					const isFocused =
						location.pathname.match(/^\/thread\/([a-z0-9-]+)$/i)?.[1] ===
							thread_id;
					console.log({ isFocused, isAtTimelineEnd, scrollEnd: ts.is_at_end });
					if (ts.is_at_end && isAtTimelineEnd) {
						if (isFocused) {
							ctx.dispatch({ do: "thread.mark_read", thread_id, delay: true });
						} else {
							ctx.dispatch({
								do: "thread.scroll_pos",
								thread_id,
								is_at_end: ts.is_at_end,
								pos: 999999,
							});
						}
					}
				});
				return;
			}
			case "thread.attachments": {
				update(
					"thread_state",
					action.thread_id,
					"attachments",
					action.attachments,
				);
				return;
			}
			case "server": {
				const msg = action.msg;
				if (msg.type === "UpsertSession") {
					if (msg.session.id === ctx.data.session?.id) {
						update("session", msg.session);
						if (!ctx.data.user) {
							ctx.client.http.GET("/api/v1/user/{user_id}", {
								params: {
									path: {
										user_id: "@self",
									},
								},
							}).then((res) => {
								const user = res.data;
								if (!user) {
									throw new Error("couldn't fetch user");
								}
								update("user", user);
								update("users", user.id, user);
							});
							ctx.dispatch({ do: "init" });
						}
					}
				} else if (msg.type === "UpsertRoom") {
					update("rooms", msg.room.id, msg.room);
				} else if (msg.type === "UpsertThread") {
					update("threads", msg.thread.id, msg.thread);
				} else if (msg.type === "UpsertMessage") {
					console.time("UpsertMessage");
					solidBatch(() => {
						const { message } = msg;
						const { id, version_id, thread_id, nonce } = message;
						update("messages", id, message);

						if (ctx.data.threads[thread_id]) {
							update("threads", thread_id, "last_version_id", version_id);
							if (id === version_id) {
								update("threads", thread_id, "message_count", (i) => i + 1);
							}
						}

						if (!ctx.data.timelines[thread_id]) {
							update("timelines", thread_id, [{ type: "hole" }, {
								type: "remote",
								message,
							}]);
						} else {
							const tl = ctx.data.timelines[thread_id];
							const item = { type: "remote" as const, message };
							if (id === version_id) {
								const idx = tl.findIndex((i) =>
									i.type === "local" && i.message.nonce === nonce
								);
								if (idx === -1) {
									update(
										"timelines",
										message.thread_id,
										(i) => [...i, item],
									);
								} else {
									update("timelines", message.thread_id, idx, item);
								}
							} else {
								update(
									"timelines",
									message.thread_id,
									(i) =>
										i.map((j) =>
											(j.type === "remote" && j.message.id === id) ? item : j
										),
								);
							}
						}

						dispatch({ do: "thread.autoscroll", thread_id });
					});
					console.timeEnd("UpsertMessage");
					// TODO: message deletions
				} else if (msg.type === "UpsertRole") {
					const role: RoleT = msg.role;
					const { room_id } = role;
					if (!ctx.data.room_roles[room_id]) update("room_roles", room_id, {});
					update("room_roles", room_id, role.id, role);
				} else if (msg.type === "UpsertMember") {
					const member: MemberT = msg.member;
					const { room_id } = member;
					if (!ctx.data.room_members[room_id]) {
						update("room_members", room_id, {});
					}
					update("users", member.user.id, member.user);
					update("room_members", room_id, member.user.id, member);
				} else if (msg.type === "UpsertInvite") {
					const invite: InviteT = msg.invite;
					update("invites", invite.code, invite);
				} else if (msg.type === "DeleteMember") {
					const { user_id, room_id } = msg;
					update(
						"room_members",
						room_id,
						produce((obj) => {
							if (!obj) return;
							delete obj[user_id];
						}),
					);
					if (user_id === ctx.data.user?.id) {
						update(
							"rooms",
							produce((obj) => {
								delete obj[room_id];
							}),
						);
					}
				} else if (msg.type === "DeleteInvite") {
					const { code } = msg;
					update(
						"invites",
						produce((obj) => {
							delete obj[code];
						}),
					);
				} else if (msg.type === "UpsertUser") {
					const { user } = msg;
					update("users", user.id, user);
					if (user.id === ctx.data.user?.id) {
						update("user", user);
					}
				} else {
					console.warn("unknown message", msg);
				}
				return;
			}
			case "server.ready": {
				const { user, session } = action.msg;
				if (user) {
					update("user", user);
					update("users", user.id, user);
				}
				update("session", session);
				return;
			}
			case "thread.mark_read": {
				const { thread_id, delay, also_local } = action;
				// NOTE: may need separate timeouts per thread
				clearTimeout(ackGraceTimeout);
				clearTimeout(ackDebounceTimeout);
				if (delay) {
					ackGraceTimeout = setTimeout(() => {
						ackDebounceTimeout = setTimeout(() => {
							ctx.dispatch({ ...action, delay: false });
						}, 800);
					}, 200);
					return;
				}

				const version_id = action.version_id ??
					ctx.data.threads[thread_id].last_version_id;
				await ctx.client.http.PUT("/api/v1/thread/{thread_id}/ack", {
					params: { path: { thread_id } },
					body: { version_id },
				});
				update("threads", thread_id, "last_read_id", version_id);
				const has_thread = !!ctx.data.thread_state[action.thread_id];
				if (also_local && has_thread) {
					update(
						"thread_state",
						action.thread_id,
						"read_marker_id",
						version_id,
					);
				}
				return;
			}
			case "fetch.room": {
				const { data, error } = await ctx.client.http.GET(
					"/api/v1/room/{room_id}",
					{
						params: { path: { room_id: action.room_id } },
					},
				);
				if (error) throw error;
				update("rooms", action.room_id, data);
				return;
			}
			case "fetch.thread": {
				const { data, error } = await ctx.client.http.GET(
					"/api/v1/thread/{thread_id}",
					{
						params: { path: { thread_id: action.thread_id } },
					},
				);
				if (error) throw error;
				update("threads", action.thread_id, data);
				return;
			}
			case "fetch.room_threads": {
				// TODO: paginate
				const { data, error } = await ctx.client.http.GET(
					"/api/v1/room/{room_id}/thread",
					{
						params: {
							path: { room_id: action.room_id },
							query: {
								dir: "f",
								limit: 100,
							},
						},
					},
				);
				if (error) throw error;
				solidBatch(() => {
					for (const item of data.items) {
						update("threads", item.id, item);
					}
				});
				return;
			}
			case "upload.init": {
				const { local_id, thread_id } = action;
				const ts = () => ctx.data.thread_state[thread_id];
				update(
					"thread_state",
					thread_id,
					"attachments",
					ts().attachments.length,
					{
						status: "uploading",
						file: action.file,
						local_id: local_id,
						progress: 0,
						paused: false,
					},
				);
				const up = await createUpload({
					file: action.file,
					client: ctx.client,
					onProgress(progress) {
						const idx = ts().attachments.findIndex((i) =>
							i.local_id === local_id
						);
						if (idx === -1) return;
						update("thread_state", thread_id, "attachments", idx, {
							status: "uploading",
							file: action.file,
							local_id,
							paused: false,
							progress,
						});
					},
					onFail(error) {
						const idx = ts().attachments.findIndex((i) =>
							i.local_id === local_id
						);
						if (idx === -1) return;
						update(
							"thread_state",
							thread_id,
							"attachments",
							ts().attachments.toSpliced(idx, 1),
						);
						ctx.dispatch({ do: "modal.alert", text: error.message });
					},
					onComplete(media) {
						const idx = ts().attachments.findIndex((i) =>
							i.local_id === local_id
						);
						if (idx === -1) return;
						update("thread_state", thread_id, "attachments", idx, {
							status: "uploaded",
							media,
							local_id,
							file: action.file,
						});
					},
					onPause() {
						const idx = ts().attachments.findIndex((i) =>
							i.local_id === local_id
						);
						if (idx === -1) return;
						update("thread_state", thread_id, "attachments", idx, {
							...ctx.data.thread_state[thread_id].attachments[idx],
							paused: true,
						});
					},
					onResume() {
						const idx = ts().attachments.findIndex((i) =>
							i.local_id === local_id
						);
						if (idx === -1) return;
						update("thread_state", thread_id, "attachments", idx, {
							...ctx.data.thread_state[thread_id].attachments[idx],
							paused: false,
						});
					},
				});
				update("uploads", local_id, { up, thread_id });
				return;
			}
			case "upload.pause": {
				ctx.data.uploads[action.local_id]?.up.pause();
				return;
			}
			case "upload.resume": {
				ctx.data.uploads[action.local_id]?.up.resume();
				return;
			}
			case "upload.cancel": {
				const upload = ctx.data.uploads[action.local_id];
				upload?.up.pause();
				delete ctx.data.uploads[action.local_id];
				const ts = ctx.data.thread_state[upload.thread_id];
				const idx = ts.attachments.findIndex((i) =>
					i.local_id === action.local_id
				);
				if (idx !== -1) {
					ctx.dispatch({
						do: "thread.attachments",
						thread_id: upload.thread_id,
						attachments: ts.attachments.toSpliced(idx, 1),
					});
				}
				return;
			}
			case "init": {
				const { data, error } = await ctx.client.http.GET("/api/v1/room", {
					params: {
						query: {
							dir: "f",
							limit: 100,
						},
					},
				});
				if (error) {
					// TODO: handle unauthenticated
					// console.error(error);
					return;
				}
				solidBatch(() => {
					for (const room of data.items) {
						update("rooms", room.id, room);
					}
				});
				return;
			}
			case "server.init_session": {
				const res = await ctx.client.http.POST("/api/v1/session", {
					body: {},
				});
				if (!res.data) {
					console.log("failed to init session", res.response);
					throw new Error("failed to init session");
				}
				const session = res.data;
				localStorage.setItem("token", session.token);
				update("session", session);
				ctx.client.start(session.token);
				return;
			}
		}
	}

	return dispatch;
}

// TODO: implement a retry queue
// TODO: show when messages fail to send
async function handleSubmit(
	ctx: ChatCtx,
	thread_id: string,
	text: string,
	update: SetStoreFunction<Data>,
) {
	if (text.startsWith("/")) {
		const [cmd, ...args] = text.slice(1).split(" ");
		const { room_id } = ctx.data.threads[thread_id];
		if (cmd === "thread") {
			const name = text.slice("/thread ".length);
			await ctx.client.http.POST("/api/v1/room/{room_id}/thread", {
				params: { path: { room_id } },
				body: { name },
			});
		} else if (cmd === "archive") {
			await ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
				body: {
					is_closed: true,
				},
			});
		} else if (cmd === "unarchive") {
			await ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
				body: {
					is_closed: false,
				},
			});
		} else if (cmd === "desc") {
			const description = args.join(" ");
			await ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
				body: {
					description: description || null,
				},
			});
		} else if (cmd === "name") {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http.PATCH("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
				body: { name },
			});
		} else if (cmd === "desc-room") {
			const description = args.join(" ");
			await ctx.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body: {
					description: description || null,
				},
			});
		} else if (cmd === "name-room") {
			const name = args.join(" ");
			if (!name) return;
			await ctx.client.http.PATCH("/api/v1/room/{room_id}", {
				params: { path: { room_id } },
				body: { name },
			});
		}
		return;
	}
	const ts = ctx.data.thread_state[thread_id];
	if (text.length === 0 && ts.attachments.length === 0) return false;
	if (!ts.attachments.every((i) => i.status === "uploaded")) return false;
	const attachments = ts.attachments.map((i) => i.media);
	const reply_id = ts.reply_id;
	const nonce = uuidv7();
	ctx.client.http.POST("/api/v1/thread/{thread_id}/message", {
		params: {
			path: { thread_id },
		},
		body: {
			content: text,
			reply_id,
			nonce,
			attachments,
		},
	});
	const localMessage: MessageT = {
		type: MessageType.Default,
		id: nonce,
		thread_id,
		version_id: nonce,
		override_name: null,
		reply_id,
		nonce,
		content: text,
		author: ctx.data.user!,
		metadata: null,
		attachments,
		is_pinned: false,
		ordering: 0,
	};
	solidBatch(() => {
		update(
			"timelines",
			thread_id,
			(i) => [...i, { type: "local" as const, message: localMessage }],
		);
		// TODO: is this necessary?
		// update("messages", msg.id, msg);
		update("thread_state", thread_id, "reply_id", null);
		update("thread_state", thread_id, "attachments", []);
		ctx.dispatch({ do: "thread.autoscroll", thread_id });
	});
}
