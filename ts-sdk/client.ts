import createFetch from "openapi-fetch";
import * as oapi from "openapi-fetch";
import type { paths } from "./schema.d.ts";
import {
	MessageEnvelope,
	MessageReady,
	MessageSync,
	Pagination,
	PaginationQuery,
	Room,
	Thread,
	User,
} from "./types.ts";
import {
	createObservable,
	createObservableMap,
	ObservableMap,
	Observer,
} from "./observable.ts";
import { uuidv7 } from "uuidv7";
export * from "./observable.ts";

export type ClientState = "stopped" | "connected" | "ready" | "reconnecting";

export type ClientOptions = {
	baseUrl: string;
	token?: string;
	onReady: (event: MessageReady) => void;
	onSync: (event: MessageSync) => void;
};

export type Http = oapi.Client<paths>;

export type Client = {
	opts: ClientOptions;

	/** Typed fetch */
	http: Http;

	/** Start receiving events */
	start: (token?: string) => void;

	/** Stop receiving events */
	stop: () => void;

	state: Observer<ClientState>;

	rooms: Rooms;
	threads: Threads;
	users: CachedMap<User>;
};

type Resume = {
	conn: string;
	seq: number;
};

export function createClient(opts: ClientOptions): Client {
	let ws: WebSocket;
	let resume: null | Resume = null;
	const state = createObservable<ClientState>("stopped");

	const http = createFetch<paths>({
		baseUrl: opts.baseUrl,
	});

	http.use({
		onRequest(r) {
			if (opts.token) {
				r.request.headers.set("authorization", `Bearer ${opts.token}`);
			}
			return r.request;
		},
	});

	const rooms = createRooms(http);
	const threads = createThreads(http);

	const users = createCached(async (user_id) => {
		const { data, error } = await http.GET("/api/v1/user/{user_id}", {
			params: { path: { user_id } },
		});
		const entry: CachedEntry<User> = error
			? { status: "error", error }
			: { status: "loaded", remote: data, local: data };
		if (error) console.error(error);
		return entry;
	});

	function setState(newState: ClientState) {
		state.set(newState);
	}

	function setupWebsocket() {
		if (state.get() !== "reconnecting") return;

		ws = new WebSocket(new URL("/api/v1/sync", opts.baseUrl));
		ws.addEventListener("message", (e) => {
			const msg: MessageEnvelope = JSON.parse(e.data);
			if (msg.op === "Ping") {
				ws.send(JSON.stringify({ type: "Pong" }));
			} else if (msg.op === "Sync") {
				if (resume) resume.seq = msg.seq;
				opts.onSync(msg.data);
				handleSync(msg.data);
			} else if (msg.op === "Error") {
				console.error(msg.error);
				setState("reconnecting");
				ws.close();
			} else if (msg.op === "Ready") {
				opts.onReady(msg);
				resume = { conn: msg.conn, seq: msg.seq };
				setState("ready");
			} else if (msg.op === "Resumed") {
				setState("ready");
			} else if (msg.op === "Reconnect") {
				if (!msg.can_resume) resume = null;
				ws.close();
			}
		});

		ws.addEventListener("open", (_e) => {
			setState("connected");
			ws.send(JSON.stringify({ type: "Hello", token: opts.token, ...resume }));
		});

		ws.addEventListener("error", (e) => {
			setState("reconnecting");
			console.error(e);
			ws.close();
		});

		ws.addEventListener("close", () => {
			setTimeout(setupWebsocket, 1000);
		});
	}

	function handleSync(s: MessageSync) {
		if (s.type === "UpsertRoom") {
			const { room } = s;
			const ent = { status: "loaded" as const, local: room, remote: room };
			rooms._cache.write.set(room.id, ent);
		} else if (s.type === "UpsertThread") {
			const { thread } = s;
			const ent = { status: "loaded" as const, local: thread, remote: thread };
			threads._cache.write.set(thread.id, ent);
		} else if (s.type === "UpsertUser") {
			const { user } = s;
			const ent = { status: "loaded" as const, local: user, remote: user };
			users._cache.write.set(user.id, ent);
		} else if (s.type === "DeleteUser") {
			const { id } = s;
			users._cache.write.set(id, { status: "missing" });
		}
	}

	function start(token?: string) {
		if (token) opts.token = token;
		setState("reconnecting");
		if (ws) {
			ws.close();
		} else {
			setupWebsocket();
		}
	}

	function stop() {
		setState("stopped");
		ws?.close();
	}

	return {
		state: state.observable,
		opts,
		http,
		start,
		stop,
		rooms,
		threads,
		users,
	};
}

type CachedEntry<T> =
	| { status: "loaded"; remote: T; local: T }
	| { status: "updating"; remote: T; local: T }
	| { status: "error"; error: string }
	| { status: "loading" }
	| { status: "missing" };

type CachedMap<T> = {
	_cache: ObservableMap<string, CachedEntry<T>>;
	watch(id: string): Observer<CachedEntry<T>>;
	fetch(id: string): Promise<CachedEntry<T>>;
};

function createCached<T>(
	fetch: (id: string) => Promise<CachedEntry<T>>,
): CachedMap<T> {
	const cache = createObservableMap<string, CachedEntry<T>>({
		status: "loading",
	});

	const fetchAndCache = async (id: string) => {
		const r = await fetch(id);
		cache.write.set(id, r);
		return r;
	};

	return {
		_cache: cache,

		watch(id: string): Observer<CachedEntry<T>> {
			const isCached = cache.read.has(id);
			const o = cache.read.watch(id);
			if (!isCached) fetchAndCache(id);
			return o;
		},

		async fetch(id: string): Promise<CachedEntry<T>> {
			const isCached = cache.read.has(id);
			if (!isCached) {
				return await fetchAndCache(id);
			} else {
				return cache.read.get(id);
			}
		},
	};
}

function createListable<T extends { id: string }>(
	map: CachedMap<T>,
	fetch: (query: PaginationQuery) => Promise<Pagination<T> | null>,
) {
	let hasAllFrom = UUID_MIN; // FIXME: disconnects, connection lag
	let hasAllTo = uuidv7();

	return {
		async fetch() {
			const data = await fetch({
				dir: "f",
				limit: 10,
				from: hasAllFrom,
				to: hasAllTo,
			});
			if (!data) return;
			for (const item of data.items) {
				map._cache.write.set(item.id, {
					status: "loaded",
					local: item,
					remote: item,
				});
			}
			if (data.has_more) {
				// panics if limit = 0
				hasAllTo = data.items.at(-1)!.id;
			} else {
				hasAllFrom = UUID_MIN;
			}
		},
	};
}

// class Ranges {
// 	add(from, to)
// 	del(from, to)
// 	has(id)
// }

// type Rooms = CachedMap<Room>;
type Rooms = ReturnType<typeof createRooms>;
type Threads = ReturnType<typeof createThreads>;

function createRooms(http: Http) {
	const map = createCached(async (room_id) => {
		const { data, error } = await http.GET("/api/v1/room/{room_id}", {
			params: { path: { room_id } },
		});
		if (error) console.error(error);
		const entry: CachedEntry<Room> = error
			? { status: "error", error }
			: { status: "loaded", remote: data, local: data };
		return entry;
	});

	const list = createListable(map, async (q) => {
		const { data, error } = await http.GET("/api/v1/room", {
			params: { query: q },
		});
		if (error) {
			console.error(error);
			return null;
		} else {
			return data;
		}
	});

	return { ...map, list: () => list.fetch() };
}

function createThreads(http: Http) {
	const map = createCached(async (thread_id) => {
		const { data, error } = await http.GET("/api/v1/thread/{thread_id}", {
			params: { path: { thread_id } },
		});
		if (error) console.error(error);
		const entry: CachedEntry<Thread> = error
			? { status: "error", error }
			: { status: "loaded", remote: data, local: data };
		return entry;
	});

	const _listCache = new Map();
	const listInRoom = (room_id: string) => {
		const existing = _listCache.get(room_id);
		if (existing) return existing.fetch();
		const list = createListable(map, async (q) => {
			const { data, error } = await http.GET("/api/v1/room/{room_id}/thread", {
				params: { path: { room_id }, query: q },
			});
			if (error) {
				console.error(error);
				return null;
			} else {
				return data;
			}
		});
		_listCache.set(room_id, list);
		return list.fetch();
	};

	return { ...map, listInRoom };
}

export const UUID_MIN = "00000000-0000-0000-0000-000000000000";
export const UUID_MAX = "ffffffff-ffff-ffff-ffff-ffffffffffff";
