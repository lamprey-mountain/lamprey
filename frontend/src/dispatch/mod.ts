import { produce, reconcile, SetStoreFunction } from "solid-js/store";
import { Action, Attachment, Data, Menu } from "../context.ts";
import { batch as solidBatch } from "solid-js";
import { ChatCtx } from "../context.ts";
import { createEditorState } from "../Editor.tsx";
import { createUpload } from "sdk";
import {
	calculateSlice,
	dispatchMessages,
	renderTimeline,
} from "./messages.ts";
import { handleSubmit } from "./submit.ts";
import { dispatchServer } from "./server.ts";
import { MessageReady } from "../../../ts-sdk/types.ts";

type Reduction =
	| { do: "menu"; menu: Menu | null }
	| { do: "modal.close" }
	| { do: "modal.alert"; text: string }
	| { do: "modal.prompt"; text: string; cont: (text: string | null) => void }
	| { do: "modal.confirm"; text: string; cont: (confirmed: boolean) => void }
	| { do: "thread.reply"; thread_id: string; reply_id: string | null }
	| {
		do: "thread.scroll_pos";
		thread_id: string;
		pos: number | null;
		is_at_end: boolean;
	}
	| {
		do: "thread.attachments";
		thread_id: string;
		attachments: Array<Attachment>;
	}
	| { do: "server.ready"; msg: MessageReady }
	| { do: "menu.preview"; id: string };

function reduce(
	state: Data,
	delta: Reduction,
): Data {
	switch (delta.do) {
		case "menu": {
			return { ...state, menu: delta.menu };
		}
		case "modal.close": {
			return { ...state, modals: state.modals.slice(1) };
		}
		case "modal.alert": {
			return {
				...state,
				modals: [{ type: "alert", text: delta.text }, ...state.modals],
			};
		}
		case "modal.prompt": {
			const modal = {
				type: "prompt" as const,
				text: delta.text,
				cont: delta.cont,
			};
			return { ...state, modals: [modal, ...state.modals] };
		}
		case "modal.confirm": {
			const modal = {
				type: "confirm" as const,
				text: delta.text,
				cont: delta.cont,
			};
			return { ...state, modals: [modal, ...state.modals] };
		}
		case "thread.reply": {
			return produce((s: Data) => {
				s.thread_state[delta.thread_id].reply_id = delta.reply_id;
				return s;
			})(state);
		}
		case "thread.scroll_pos": {
			return produce((s: Data) => {
				s.thread_state[delta.thread_id].scroll_pos = delta.pos;
				s.thread_state[delta.thread_id].is_at_end = delta.is_at_end;
				return s;
			})(state);
		}
		case "thread.attachments": {
			return produce((s: Data) => {
				s.thread_state[delta.thread_id].attachments = delta.attachments;
				return s;
			})(state);
		}
		case "server.ready": {
			const { user, session } = delta.msg;
			return produce((s: Data) => {
				if (user) {
					s.user = user;
					s.users[user.id] = user;
				}
				s.session = session;
			})(state);
		}
		case "menu.preview": {
			return {
				...state,
				cursor: {
					...state.cursor,
					preview: delta.id,
				},
			};
		}
	}
}

type Middleware = (
	state: Data,
	dispatch: (action: Action) => void,
) => (next: (action: Action) => void) => (action: Action) => void;

function combine(
	reduce: (state: Data, delta: Reduction) => Data,
	state: Data,
	update: SetStoreFunction<Data>,
	middleware: Array<Middleware>,
) {
	let _dispatch = (_action: Action) => {};
	const dispatch = (action: Action) => {
		console.log("reduce", state, action);
		update((s) => reduce(s, action as Reduction));
	};
	const merged = middleware.toReversed().reduce(
		(dispatch, m) => (action) => m(state, _dispatch)(dispatch)(action),
		dispatch,
	);
	_dispatch = merged;
	return merged;
}

// TODO: refactor this out into multiple smaller files
export function createDispatcher(ctx: ChatCtx, update: SetStoreFunction<Data>) {
	let ackGraceTimeout: number | undefined;
	let ackDebounceTimeout: number | undefined;

	const threadAutoscroll: Middleware =
		(_state, _dispatch) => (next) => (action) => {
			if (action.do === "thread.autoscroll") {
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
			} else {
				next(action);
			}
		};

	const threadMarkRead: Middleware =
		(_state, _dispatch) => (next) => async (action) => {
			if (action.do === "thread.mark_read") {
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
			} else {
				next(action);
			}
		};

	const fetchRoom: Middleware =
		(_state, _dispatch) => (next) => async (action) => {
			if (action.do === "fetch.room") {
				const { data, error } = await ctx.client.http.GET(
					"/api/v1/room/{room_id}",
					{
						params: { path: { room_id: action.room_id } },
					},
				);
				if (error) throw error;
				update("rooms", action.room_id, data);
			} else {
				next(action);
			}
		};

	const fetchThread: Middleware =
		(_state, _dispatch) => (next) => async (action) => {
			if (action.do === "fetch.thread") {
				const { data, error } = await ctx.client.http.GET(
					"/api/v1/thread/{thread_id}",
					{
						params: { path: { thread_id: action.thread_id } },
					},
				);
				if (error) throw error;
				update("threads", action.thread_id, data);
			} else {
				next(action);
			}
		};

	const fetchRoomThreads: Middleware =
		(_state, _dispatch) => (next) => async (action) => {
			if (action.do === "fetch.room_threads") {
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
			} else {
				next(action);
			}
		};

	const uploadInit: Middleware =
		(_state, _dispatch) => (next) => async (action) => {
			if (action.do === "upload.init") {
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
			} else {
				next(action);
			}
		};

	const uploadPause: Middleware = (_state, _dispatch) => (next) => (action) => {
		if (action.do === "upload.pause") {
			ctx.data.uploads[action.local_id]?.up.pause();
		} else {
			next(action);
		}
	};

	const uploadResume: Middleware =
		(_state, _dispatch) => (next) => (action) => {
			if (action.do === "upload.resume") {
				ctx.data.uploads[action.local_id]?.up.resume();
			} else {
				next(action);
			}
		};

	const uploadCancel: Middleware =
		(_state, _dispatch) => (next) => (action) => {
			if (action.do === "upload.cancel") {
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
			} else {
				next(action);
			}
		};

	const init: Middleware = (_state, _dispatch) => (next) => async (action) => {
		if (action.do === "init") {
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
		} else {
			next(action);
		}
	};

	const serverInitSession: Middleware =
		(_state, _dispatch) => (next) => async (action) => {
			if (action.do === "server.init_session") {
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
			} else {
				next(action);
			}
		};

	const mouseMoved: Middleware = (_state, _dispatch) => (next) => (action) => {
		if (action.do === "window.mouse_move") {
			// TODO: use triangle to submenu corners instead of dot with x axis
			const pos = [
				...ctx.data.cursor.pos,
				[action.e.x, action.e.y] as [number, number],
			];
			if (pos.length > 5) pos.shift();
			let vx = 0, vy = 0;
			for (let i = 1; i < pos.length; i++) {
				vx += pos[i - 1][0] - pos[i][0];
				vy += pos[i - 1][1] - pos[i][1];
			}
			solidBatch(() => {
				update("cursor", "pos", pos);
				update("cursor", "vel", (vx / Math.hypot(vx, vy)) || 0);
			});
		} else {
			next(action);
		}
	};

	const threadSend: Middleware = (_state, _dispatch) => (next) => (action) => {
		if (action.do === "thread.send") {
			handleSubmit(ctx, action.thread_id, action.text, update);
		} else {
			next(action);
		}
	};

	const handleServer: Middleware = (_state, dispatch) => (next) => (action) => {
		if (action.do === "server") {
			return dispatchServer(ctx, update, action, dispatch);
		} else {
			next(action);
		}
	};

	const threadInit: Middleware = (state, dispatch) => (next) => (action) => {
		if (action.do === "thread.init") {
			const { thread_id } = action;
			if (state.thread_state[thread_id]) return;
			update("thread_state", thread_id, {
				editor_state: createEditorState((text) => {
					dispatch({ do: "thread.send", thread_id, text });
				}),
				reply_id: null,
				scroll_pos: null,
				read_marker_id: action.read_id ?? null,
				attachments: [],
				is_at_end: true,
				timeline: [],
			});
		} else {
			next(action);
		}
	};

	const paginate: Middleware = (_state, _dispatch) => (next) => (action) => {
		if (action.do === "paginate") {
			dispatchMessages(ctx, update, action);
		} else {
			next(action);
		}
	};

	const log: Middleware = (state, _dispatch) => (next) => (action) => {
		console.log("dispatch", action, state);
		next(action);
	};

	const d = combine(reduce, ctx.data, update, [
		log,
		threadAutoscroll,
		threadMarkRead,
		fetchRoom,
		fetchThread,
		handleServer,
		init,
		serverInitSession,
		threadInit,
		paginate,
		uploadCancel,
		uploadInit,
		uploadPause,
		uploadResume,
		fetchRoomThreads,
		mouseMoved,
		threadSend,
	]);

	return d;
}
