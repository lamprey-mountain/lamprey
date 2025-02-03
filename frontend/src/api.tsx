// TODO: this file is getting big and should probably be split and refactored
// i'm copypasting stuff for now, but will refactor out abstractions later

import {
	Accessor,
	batch,
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
import {
	MessageListAnchor,
	MessageRange,
	MessageRanges,
	Messages,
} from "./api/messages.ts";
import { Rooms } from "./api/rooms.ts";
import { Threads } from "./api/threads.ts";
import { Users } from "./api/users.ts";

export type Json =
	| number
	| string
	| boolean
	| Array<Json>
	| { [k in string]: Json };

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

	const rooms = new Rooms();
	const threads = new Threads();
	const users = new Users();
	const messages = new Messages();

	props.temp_events.on("sync", (msg) => {
		if (msg.type === "UpsertRoom") {
			const { room } = msg;
			rooms.cache.set(room.id, room);
			if (rooms._cachedListing?.pagination) {
				const l = rooms._cachedListing;
				const p = l.pagination!;
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
		} else if (msg.type === "UpsertThread") {
			const { thread } = msg;
			threads.cache.set(thread.id, thread);
			const l = threads._cachedListings.get(thread.room_id);
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
			users.cache.set(msg.user.id, msg.user);
			if (msg.user.id === users.cache.get("@self")?.id) {
				users.cache.set("@self", msg.user);
			}
		} else if (msg.type === "UpsertSession") {
			if (msg.session?.id === session()?.id) {
				setSession(session);
			}
		} else if (msg.type === "DeleteRoomMember") {
			const user_id = users.cache.get("@self")?.id;
			if (msg.user_id === user_id) {
				if (rooms._cachedListing?.pagination) {
					const l = rooms._cachedListing;
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
			const r = messages.cacheRanges.get(m.thread_id);
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
				batch(() => {
					messages.cache.set(m.id, m);
					messages._updateMutators(r, m.thread_id);
				});
			}
		}
	});

	props.temp_events.on("ready", (msg) => {
		if (msg.user) {
			users.cache.set("@self", msg.user);
			users.cache.set(msg.user.id, msg.user);
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

	// FIXME: make reactive again
	const api: Api = {
		rooms,
		threads,
		users,
		messages,
		session,
		tempCreateSession,
		client: props.client,
	};
	messages.api = api;
	rooms.api = api;
	threads.api = api;
	users.api = api;

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
		fetch: (room_id: () => string) => Resource<Room>;
		list: () => Resource<Pagination<Room>>;
		cache: ReactiveMap<string, Room>;
	};
	threads: {
		fetch: (thread_id: () => string) => Resource<Thread>;
		list: (room_id: () => string) => Resource<Pagination<Thread>>;
		cache: ReactiveMap<string, Thread>;
	};
	users: {
		fetch: (user_id: () => string) => Resource<User>;
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
		fetch: (
			thread_id: () => string,
			message_id: () => string,
		) => Resource<Message>;
		cache: ReactiveMap<string, Message>;
		cacheRanges: Map<string, MessageRanges>;
	};
	session: Accessor<Session | null>;
	tempCreateSession: () => void;
	client: Client;
};

export type Listing<T> = {
	resource: Resource<Pagination<T>>;
	pagination: Pagination<T> | null;
	mutate: (value: Pagination<T>) => void;
	refetch: () => void;
	prom: Promise<unknown> | null;
};
