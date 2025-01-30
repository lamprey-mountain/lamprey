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

export function createReactiveResource<T>(
	fetch: (id: string) => Promise<ResourceResponse<T>>,
): [ReactiveMap<string, T>, ResourceFetch<T>] {
	const cache = new ReactiveMap<string, T>();
	const requests = new Map<string, Promise<T>>();

	function inner(id: () => string): [Resource<T>] {
		const [data, { mutate }] = createResource<T, string>(id, (id) => {
			console.log("start");
			const cached = cache.get(id);
			if (cached) return cached;
			const existing = requests.get(id);
			if (existing) return existing;

			const req = (async () => {
				const { data, error } = await fetch(id);
				if (error) throw error;
				console.log("finish");
				requests.delete(id);
				cache.set(id, data);
				createEffect(() => {
					// HACK: extra closure to make typescript happy
					mutate(() => cache.get(id));
				});
				return data;
			})();

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

	function createRoomList(): () => Resource<Pagination<Room>> {
		type T = Room;
		const cache = roomCache;
		let pagination: Pagination<T> | undefined;
		let resource: Resource<Pagination<T>>;
		// let mutate: (value: T) => void;
		let refetch: () => void;
		let isFetching = false;

		return () => {
			if (resource) {
				if (!isFetching) refetch();
				return resource;
			}

			[resource, { refetch }] = createResource(async () => {
				isFetching = true;
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
					pagination = {
						...data,
						items: [...pagination?.items ?? [], ...data.items],
					};
				});

				isFetching = false;
				return pagination!;
			});

			return resource;
		};
	}

	function createThreadList() {
		type T = Thread;
		type P = Pagination<T>;
		type Listing = {
			resource: Resource<P>;
			pagination: P | undefined;
			// mutate: (value: T) => void;
			refetch: () => void;
			prom: Promise<unknown> | undefined;
		};

		const cache = threadCache;
		const listings = new Map<string, Listing>();

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

				const listing = { isFetching: true } as unknown as Listing;
				listings.set(room_id, listing);

				const [resource, { refetch }] = createResource(
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
			});

			return listings.get(untrack(room_id_signal))!.resource;
		};
	}

	const roomList = createRoomList();
	const threadList = createThreadList();

	props.temp_events.on("sync", (msg) => {
		if (msg.type === "UpsertRoom") {
			roomCache.set(msg.room.id, msg.room);
		} else if (msg.type === "UpsertThread") {
			threadCache.set(msg.thread.id, msg.thread);
		} else if (msg.type === "UpsertUser") {
			userCache.set(msg.user.id, msg.user);
			if (msg.user.id === userCache.get("@self")?.id) {
				userCache.set("@self", msg.user);
			}
		} else if (msg.type === "UpsertSession") {
			if (msg.session?.id === session()?.id) {
				setSession(session);
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

	const api = {
		rooms: { cache: roomCache, fetch: roomFetch, list: roomList },
		threads: { cache: threadCache, fetch: threadFetch, list: threadList },
		users: { cache: userCache, fetch: userFetch },
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
	session: Accessor<Session | null>;
	tempCreateSession: () => void;
};
