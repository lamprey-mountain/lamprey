// TODO: this file is getting big and should probably be split and refactored
// i'm copypasting stuff for now, but will refactor out abstractions later

import {
	Accessor,
	batch,
	createComputed,
	createContext,
	createSignal,
	ParentProps,
	Resource,
	useContext,
} from "solid-js";
import { ReactiveMap } from "@solid-primitives/map";
import {
	Client,
	Media,
	Message,
	MessageCreate,
	MessageReady,
	MessageSync,
	Pagination,
	Room,
	Session,
	Thread,
	User,
} from "sdk";
import { Emitter } from "@solid-primitives/event-bus";
import { createResource } from "solid-js";
import { createEffect } from "solid-js";
import { untrack } from "solid-js";
import {
	MessageListAnchor,
	MessageMutator,
	MessageRange,
	MessageRanges,
} from "./api/messages.ts";
import { MessageType } from "./types.ts";
import { uuidv7 } from "uuidv7";
import { onCleanup } from "solid-js";

type ResourceResponse<T> = { data: T; error: undefined } | {
	data: undefined;
	error: Error;
};

export type Json =
	| number
	| string
	| boolean
	| Array<Json>
	| { [k in string]: Json };

type ResourceFetch<T> = (id: () => string) => [Resource<T>];

function assertEq<T>(a: T, b: T) {
	if (a !== b) throw new Error(`assert failed: ${a} !== ${b}`);
}

export function createReactiveResource<T>(
	fetch: (id: string) => Promise<ResourceResponse<T>>,
): [ReactiveMap<string, T>, ResourceFetch<T>] {
	const cache = new ReactiveMap<string, T>();
	const requests = new Map<string, Promise<T>>();

	function inner(id: () => string): [Resource<T>] {
		const [data, { mutate }] = createResource<T, string>(id, (id) => {
			const cached = cache.get(id);
			if (cached) return cached;
			const existing = requests.get(id);
			if (existing) return existing;

			const req = (async () => {
				const { data, error } = await fetch(id);
				if (error) throw error;
				requests.delete(id);
				cache.set(id, data);
				return data;
			})();
			
			createEffect(() => {
				// HACK: extra closure to make typescript happy
				mutate(() => cache.get(id));
			});

			requests.set(id, req);
			return req;
		}, {});

		return [data];
	}

	return [cache, inner];
}

const ApiContext = createContext<Api>();

export function useApi() {
	return useContext(ApiContext)!;
}

export function ApiProvider(
	props: ParentProps<{
		client: Client;
		temp_events: Emitter<{
			sync: MessageSync;
			ready: MessageReady;
		}>;
	}>,
) {
	const [session, setSession] = createSignal<Session | null>(null);

	const [roomCache, roomFetch] = createReactiveResource<Room>((room_id) => {
		console.log("fetch", room_id);
		return props.client.http.GET("/api/v1/room/{room_id}", {
			params: { path: { room_id } },
		});
	});

	const [threadCache, threadFetch] = createReactiveResource<Thread>(
		(thread_id) => {
			console.log("fetch thread", thread_id);
			return props.client.http.GET("/api/v1/thread/{thread_id}", {
				params: { path: { thread_id } },
			});
		},
	);

	const [userCache, userFetch] = createReactiveResource<User>((user_id) => {
		console.log("fetch user", user_id);
		return props.client.http.GET("/api/v1/user/{user_id}", {
			params: { path: { user_id } },
		});
	});

	type Listing<T> = {
		resource: Resource<Pagination<T>>;
		pagination: Pagination<T> | undefined;
		mutate: (value: Pagination<T>) => void;
		refetch: () => void;
		prom: Promise<unknown> | undefined;
	};

	const roomListing: Listing<Room> | Record<string, never> = {};
	const threadListings = new Map<string, Listing<Thread>>();

	function createRoomList(): () => Resource<Pagination<Room>> {
		type T = Room;
		const cache = roomCache;
		const listing = roomListing;

		async function paginate(pagination?: Pagination<T>) {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await props.client.http.GET("/api/v1/room", {
				params: {
					query: {
						dir: "f",
						limit: 100,
						from: pagination?.items.at(-1)?.id,
					},
				},
			});

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.items) {
					cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		}

		return () => {
			if (listing.resource) {
				if (!listing.prom) listing.refetch();
				return listing.resource;
			}

			const [resource, { refetch, mutate }] = createResource(async () => {
				if (listing?.prom) {
					await listing!.prom;
					return listing!.pagination!;
				}

				const prom = paginate(listing!.pagination);
				listing!.prom = prom;
				const res = await prom;
				listing!.pagination = res;
				listing!.prom = undefined;
				return res!;
			});

			listing.resource = resource;
			listing.refetch = refetch;
			listing.mutate = mutate;

			return resource;
		};
	}

	function createThreadList() {
		type T = Thread;
		type P = Pagination<T>;

		const cache = threadCache;
		const listings = threadListings;

		async function paginate(room_id: string, pagination?: P): Promise<P> {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await props.client.http.GET(
				"/api/v1/room/{room_id}/thread",
				{
					params: {
						path: { room_id },
						query: {
							dir: "f",
							limit: 100,
							from: pagination?.items.at(-1)?.id,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.items) {
					cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		}

		return (room_id_signal: () => string) => {
			createComputed(() => {
				const room_id = room_id_signal();
				const cached = listings.get(room_id);

				if (cached) {
					if (!cached.prom) cached.refetch();
					return;
				}

				const listing = { isFetching: true } as unknown as Listing<T>;
				listings.set(room_id, listing);

				const [resource, { refetch, mutate }] = createResource(
					room_id,
					async (room_id): Promise<P> => {
						if (listing.prom) {
							await listing.prom;
							return listing.pagination!;
						}

						const prom = paginate(room_id, listing.pagination);
						listing.prom = prom;
						const res = await prom;
						listing.pagination = res;
						listing.prom = undefined;
						return res!;
					},
					{},
				);

				listing.resource = resource;
				listing.refetch = refetch as () => void;
				listing.mutate = mutate;
			});

			return listings.get(untrack(room_id_signal))!.resource;
		};
	}

	const threadMessageRanges = new Map<string, MessageRanges>();
	const threadMessageMutators = new Set<MessageMutator>();

	function createMessageList() {
		return (
			thread_id_signal: () => string,
			dir_signal: () => MessageListAnchor,
		): Resource<MessageRange> => {
			// always have Ranges for the current thread
			createComputed(() => {
				const thread_id = thread_id_signal();
				const ranges = threadMessageRanges.get(thread_id) ??
					new MessageRanges();
				threadMessageRanges.set(thread_id, ranges);
			});

			let old: { thread_id: string; dir: MessageListAnchor };
			async function update(
				{ thread_id, dir }: { thread_id: string; dir: MessageListAnchor },
				{ value: oldValue }: { value?: MessageRange },
			): Promise<MessageRange> {
				// ugly, but seems to work
				if (
					old && old.thread_id === thread_id && old.dir.limit === dir.limit &&
					old.dir.type === dir.type && old.dir.message_id === dir.message_id
				) return oldValue!;
				old = { thread_id, dir };

				const ranges = threadMessageRanges.get(thread_id)!;

				console.log("recalculate message list", {
					thread_id,
					dir,
				});

				if (dir.type === "forwards") {
					if (dir.message_id) {
						const r = ranges.find(dir.message_id);
						// console.log(ranges, r);
						if (r) {
							const idx = r.items.findIndex((i) => i.id === dir.message_id);
							if (idx !== -1) {
								if (idx < r.len - dir.limit || !r.has_forward) {
									const start = idx;
									const end = Math.min(idx + dir.limit, r.len);
									const s = r.slice(start, end);
									assertEq(s.start, dir.message_id);
									return s;
								}

								throw new Error("todo");

								// // fetch more
								// const { data, error } = await props.client.http.GET(
								// 	"/api/v1/thread/{thread_id}/message",
								// 	{
								// 		params: {
								// 			path: { thread_id },
								// 			query: { dir: "b", limit: 100, from: r.start },
								// 		},
								// 	},
								// );
								// if (error) throw new Error(error);
								// for (const item of data.items.toReversed()) {
								// 	const existing = ranges.find(item.id);
								// 	if (existing) {
								// 		throw new Error("todo");
								// 	} else {
								// 		r.items.unshift(item);
								// 	}
								// }
								// r.has_backwards = data.has_more;
								// const end = idx + data.items.length + 1;
								// const start = Math.max(end - dir.limit, 0);
								// const s = r.slice(start, end);
								// assertEq(s.end, dir.message_id);
								// return s;
							} else {
								// fetch thread
								throw new Error("todo");
							}
						} else {
							// new range
							throw new Error("todo");
						}
					} else {
						throw new Error("todo");
					}
				} else if (dir.type === "backwards") {
					if (dir.message_id) {
						const r = ranges.find(dir.message_id);
						// console.log(ranges, r);
						if (r) {
							const idx = r.items.findIndex((i) => i.id === dir.message_id);
							if (idx !== -1) {
								if (idx >= dir.limit) {
									const end = idx + 1;
									const start = Math.max(end - dir.limit, 0);
									const s = r.slice(start, end);
									assertEq(s.end, dir.message_id);
									return s;
								}

								// fetch more
								const { data, error } = await props.client.http.GET(
									"/api/v1/thread/{thread_id}/message",
									{
										params: {
											path: { thread_id },
											query: { dir: "b", limit: 100, from: r.start },
										},
									},
								);
								if (error) throw new Error(error);
								for (const item of data.items.toReversed()) {
									const existing = ranges.find(item.id);
									if (existing) {
										throw new Error("todo");
									} else {
										r.items.unshift(item);
									}
								}
								r.has_backwards = data.has_more;
								const end = idx + data.items.length + 1;
								const start = Math.max(end - dir.limit, 0);
								const s = r.slice(start, end);
								assertEq(s.end, dir.message_id);
								return s;
							} else {
								// fetch thread
								throw new Error("todo");
							}
						} else {
							// new range
							throw new Error("todo");
						}
					}

					const range = ranges.live;
					if (range.isEmpty()) {
						const { data, error } = await props.client.http.GET(
							"/api/v1/thread/{thread_id}/message",
							{
								params: {
									path: { thread_id },
									query: { dir: "b", limit: 100 },
								},
							},
						);
						if (error) throw new Error(error);
						for (const item of data.items.toReversed()) {
							const existing = ranges.find(item.id);
							if (existing) {
								throw new Error("todo");
							} else {
								range.items.unshift(item);
							}
						}
						range.has_backwards = data.has_more;
					} else {
						// don't need to do anything
					}

					const start = Math.max(range.len - dir.limit, 0);
					const end = Math.min(start + dir.limit, range.len);
					return range.slice(start, end);
				} else {
					throw new Error("todo");
				}
			}

			const query = () => ({
				thread_id: thread_id_signal(),
				dir: dir_signal(),
			});

			const [resource, { mutate }] = createResource(query, update);

			// HACK: surely there's a better way...
			const mut = { mutate } as unknown as MessageMutator;
			createComputed(() => {
				mut.query = dir_signal();
				mut.thread_id = thread_id_signal();
			});
			threadMessageMutators.add(mut);
			onCleanup(() => {
				threadMessageMutators.delete(mut);
			});

			return resource;
		};
	}

	const roomList = createRoomList();
	const threadList = createThreadList();
	const messageList = createMessageList();

	props.temp_events.on("sync", (msg) => {
		if (msg.type === "UpsertRoom") {
			const { room } = msg;
			roomCache.set(room.id, room);
			if ("pagination" in roomListing) {
				const l = roomListing as unknown as Listing<Room>;
				if (l?.pagination) {
					const p = l.pagination;
					const idx = p.items.findIndex((i) => i.id === room.id);
					if (idx !== -1) {
						l.mutate({
							...p,
							items: p.items.toSpliced(idx, 1, room),
						});
					} else if (p.items.length === 0 || room.id > p.items[0].id) {
						l.mutate({
							...p,
							items: [...p.items, room],
							total: p.total + 1,
						});
					}
				}
			}
		} else if (msg.type === "UpsertThread") {
			const { thread } = msg;
			threadCache.set(thread.id, thread);
			const l = threadListings.get(thread.room_id);
			if (l?.pagination) {
				const p = l.pagination;
				const idx = p.items.findIndex((i) => i.id === thread.id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, thread),
					});
				} else if (p.items.length === 0 || thread.id > p.items[0].id) {
					l.mutate({
						...p,
						items: [...p.items, thread],
						total: p.total + 1,
					});
				}
			}
		} else if (msg.type === "UpsertUser") {
			userCache.set(msg.user.id, msg.user);
			if (msg.user.id === userCache.get("@self")?.id) {
				userCache.set("@self", msg.user);
			}
		} else if (msg.type === "UpsertSession") {
			if (msg.session?.id === session()?.id) {
				setSession(session);
			}
		} else if (msg.type === "DeleteRoomMember") {
			const user_id = userCache.get("@self")?.id;
			if (msg.user_id === user_id) {
				if ("pagination" in roomListing) {
					const l = roomListing as unknown as Listing<Room>;
					if (l?.pagination) {
						const p = l.pagination;
						const idx = p.items.findIndex((i) => i.id === msg.room_id);
						if (idx !== -1) {
							l.mutate({
								...p,
								items: p.items.toSpliced(idx, 1),
							});
						}
					}
				}
			}
		} else if (msg.type === "UpsertMessage") {
			const m = msg.message;
			const r = threadMessageRanges.get(m.thread_id);
			if (r) {
				if (m.nonce) {
					// local echo
					const idx = r.live.items.findIndex((i) => i.nonce === m.nonce);
					if (idx !== -1) {
						r.live.items.splice(idx, 1);
					}
				} else if (m.version_id !== m.id) {
					// edits
					const idx = r.live.items.findIndex((i) => i.id === m.id);
					if (idx !== -1) {
						r.live.items.splice(idx, 1);
					}
				}
				r.live.items.push(m);
				updateMessageMutators(r, m.thread_id);
			}
		}
	});

	props.temp_events.on("ready", (msg) => {
		if (msg.user) {
			userCache.set("@self", msg.user);
		}
		setSession(msg.session);
	});

	async function tempCreateSession() {
		const res = await props.client.http.POST("/api/v1/session", {
			body: {},
		});
		if (!res.data) {
			console.error("failed to init session", res.response);
			throw new Error("failed to init session");
		}
		const session = res.data;
		localStorage.setItem("token", session.token);
		setSession(session);
		props.client.start(session.token);
	}

	async function messageSend(
		thread_id: string,
		body: MessageSendReq,
	): Promise<Message> {
		const id = uuidv7();
		const local: Message = {
			type: MessageType.Default,
			id,
			thread_id,
			version_id: id,
			override_name: null,
			reply_id: null,
			content: null,
			author: api.users.cache.get("@self")!,
			metadata: null,
			is_pinned: false,
			ordering: 0,
			...body,
			nonce: id,
			is_local: true,
		};

		const r = threadMessageRanges.get(thread_id);
		if (r) {
			r.live.items.push(local);
			updateMessageMutators(r, thread_id);
		}

		const { data, error } = await props.client.http.POST(
			"/api/v1/thread/{thread_id}/message",
			{
				params: {
					path: { thread_id },
				},
				body: {
					...body,
					attachments: body.attachments?.map((i) => ({ id: i.id })),
					nonce: id,
				},
			},
		);
		if (error) throw new Error(error);
		return data;
	}

	function updateMessageMutators(r: MessageRanges, thread_id: string) {
		for (const mut of threadMessageMutators) {
			if (mut.thread_id !== thread_id) continue;
			if (mut.query.type !== "backwards") continue;
			if (mut.query.message_id) continue;
			const start = Math.max(r.live.len - mut.query.limit, 0);
			const end = Math.min(start + mut.query.limit, r.live.len);
			mut.mutate(r.live.slice(start, end));
		}
	}

	const api = {
		rooms: { cache: roomCache, fetch: roomFetch, list: roomList },
		threads: { cache: threadCache, fetch: threadFetch, list: threadList },
		users: { cache: userCache, fetch: userFetch },
		messages: { list: messageList, send: messageSend },
		session,
		tempCreateSession,
	};

	console.log("provider created", api);
	return (
		<ApiContext.Provider value={api}>
			{props.children}
		</ApiContext.Provider>
	);
}

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};

export type Api = {
	rooms: {
		fetch: ResourceFetch<Room>;
		list: () => Resource<Pagination<Room>>;
		cache: ReactiveMap<string, Room>;
	};
	threads: {
		fetch: ResourceFetch<Thread>;
		list: (room_id: () => string) => Resource<Pagination<Thread>>;
		cache: ReactiveMap<string, Thread>;
	};
	users: {
		fetch: ResourceFetch<User>;
		cache: ReactiveMap<string, User>;
	};
	messages: {
		send: (
			thread_id: string,
			message: MessageSendReq,
		) => Promise<Message>;
		list: (
			thread_id: () => string,
			anchor: () => MessageListAnchor,
		) => Resource<MessageRange>;
	};
	session: Accessor<Session | null>;
	tempCreateSession: () => void;
};
