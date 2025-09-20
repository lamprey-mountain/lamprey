// TODO: this file is getting big and should probably be split and refactored
// i'm copypasting stuff for now, but will refactor out abstractions later

import {
	type Accessor,
	batch,
	type Component,
	createContext,
	createSignal,
	type ParentProps,
	type Resource,
	useContext,
} from "solid-js";
import { ReactiveMap } from "@solid-primitives/map";
import type {
	AuditLogEntry,
	Client,
	EmojiCustom,
	Invite,
	InviteWithMetadata,
	Media,
	Message,
	MessageCreate,
	MessageReady,
	MessageSync,
	Pagination,
	Role,
	Room,
	RoomMember,
	Session,
	Thread,
	ThreadMember,
	UserWithRelationship,
	VoiceState,
} from "sdk";
import type { Emitter } from "@solid-primitives/event-bus";
import {
	type MessageListAnchor,
	type MessageRange,
	type MessageRanges,
	Messages,
} from "./api/messages.ts";
import { Rooms } from "./api/rooms.ts";
import { Threads } from "./api/threads.ts";
import { Users } from "./api/users.ts";
import { Invites } from "./api/invite.ts";
import { RoomMembers } from "./api/room_members.ts";
import { Roles } from "./api/roles.ts";
import { AuditLogs } from "./api/audit_log.ts";
import { ThreadMembers } from "./api/thread_members.ts";
import { MediaInfo } from "./api/media.tsx";
import { Emoji } from "./api/emoji.ts";
import { Dms } from "./api/dms.ts";

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

export function createApi(
	client: Client,
	events: Emitter<{
		sync: MessageSync;
		ready: MessageReady;
	}>,
) {
	const [session, setSession] = createSignal<Session | null>(null);

	const rooms = new Rooms();
	const threads = new Threads();
	const invites = new Invites();
	const roles = new Roles();
	const room_members = new RoomMembers();
	const thread_members = new ThreadMembers();
	const users = new Users();
	const messages = new Messages();
	const media = new MediaInfo();
	const typing = new ReactiveMap<string, Set<string>>();
	const typing_timeout = new Map<string, Map<string, NodeJS.Timeout>>();
	const audit_logs = new AuditLogs();
	const emoji = new Emoji();
	const dms = new Dms();
	const voiceStates = new ReactiveMap();
	const [voiceState, setVoiceState] = createSignal();

	events.on("sync", (msg) => {
		if (msg.type === "RoomCreate" || msg.type === "RoomUpdate") {
			const { room } = msg;
			rooms.cache.set(room.id, room);
			if (rooms._cachedListing?.pagination) {
				const l = rooms._cachedListing;
				const p = l.pagination!;
				const idx = p.items.findIndex((i) => i.id === room.id);
				if (idx === -1) {
					l.mutate({
						...p,
						items: [...p.items, room],
						total: p.total + 1,
					});
				} else {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, room),
					});
				}
			}
		} else if (msg.type === "ThreadCreate") {
			const { thread } = msg;
			threads.cache.set(thread.id, thread);
			if (thread.room_id) {
				const l = threads._cachedListings.get(thread.room_id);
				if (l?.pagination) {
					const p = l.pagination;
					for (const mut of threads._listingMutators) {
						if (mut.room_id === thread.room_id) {
							mut.mutate({
								...p,
								items: [...p.items, thread],
								total: p.total + 1,
							});
						}
					}
				}
			}
		} else if (msg.type === "ThreadUpdate") {
			const { thread } = msg;
			threads.cache.set(thread.id, thread);
			if (thread.room_id) {
				const l = threads._cachedListings.get(thread.room_id);
				if (l?.pagination) {
					const p = l.pagination;
					const idx = p.items.findIndex((i) => i.id === thread.id);
					if (idx !== -1) {
						for (const mut of threads._listingMutators) {
							if (mut.room_id === thread.room_id) {
								mut.mutate({
									...p,
									items: p.items.toSpliced(idx, 1, thread),
								});
							}
						}
					}
				}
			}
		} else if (msg.type === "ThreadTyping") {
			const { thread_id, user_id, until } = msg;
			const t = typing.get(thread_id) ?? new Set();
			typing.set(thread_id, new Set([...t, user_id]));

			const timeout = setTimeout(() => {
				console.log("remove typing");
				const t = typing.get(thread_id)!;
				t.delete(user_id);
				typing.set(thread_id, new Set(t));
			}, Date.parse(until) - Date.now());

			const tt = typing_timeout.get(thread_id);
			if (tt) {
				const tu = tt.get(user_id);
				if (tu) clearTimeout(tu);
				tt.set(user_id, timeout);
			} else {
				const tt = new Map();
				tt.set(user_id, timeout);
				typing_timeout.set(thread_id, tt);
			}
		} else if (msg.type === "ThreadAck") {
			// TODO
		} else if (msg.type === "MessageCreate") {
			const m = msg.message;
			const r = messages.cacheRanges.get(m.thread_id);
			let is_new = false;
			let is_unread = true;
			if (r) {
				if (m.nonce) {
					// local echo
					console.log("Message Create local echo");
					const idx = r.live.items.findIndex((i) => i.nonce === m.nonce);
					if (idx !== -1) {
						r.live.items.splice(idx, 1);
					}
					r.live.items.push(m);
					is_new = true;
					is_unread = false;
				} else {
					console.log("Message Create new message");
					r.live.items.push(m);
					is_new = true;
				}
				batch(() => {
					messages.cache.set(m.id, m);
					messages._updateMutators(r, m.thread_id);
				});
			}

			const t = api.threads.cache.get(m.thread_id);
			if (t) {
				api.threads.cache.set(m.thread_id, {
					...t,
					message_count: (t.message_count ?? 0) + (is_new ? 1 : 0),
					last_version_id: m.version_id,
					is_unread,
				});
			}

			{
				const t = typing.get(m.thread_id);
				if (t) {
					t.delete(m.author_id);
					typing.set(m.thread_id, new Set(t));
					clearTimeout(typing_timeout.get(m.thread_id)!.get(m.author_id));
				}
			}

			for (
				const att of m.type === "DefaultMarkdown" ? m.attachments ?? [] : []
			) {
				media.cacheInfo.set(att.id, att);
			}
		} else if (msg.type === "MessageUpdate") {
			const m = msg.message;
			const r = messages.cacheRanges.get(m.thread_id);
			if (r) {
				const idx = r.live.items.findIndex((i) => i.id === m.id);
				if (idx !== -1) {
					console.log("Message Update edit");
					r.live.items.splice(idx, 1, m);
				}
				batch(() => {
					messages.cache.set(m.id, m);
					messages._updateMutators(r, m.thread_id);
				});
			}
		} else if (msg.type === "MessageDelete") {
			batch(() => {
				const { message_id, thread_id } = msg;
				const ranges = messages.cacheRanges.get(thread_id);
				const r = ranges?.find(message_id);
				if (ranges && r) {
					const idx = r.items.findIndex((i) => i.id === message_id);
					if (idx !== -1) {
						r.items.splice(idx, 1);
					}
					batch(() => {
						messages.cache.delete(thread_id);
						messages._updateMutators(ranges, thread_id);
					});
				}
				const t = api.threads.cache.get(msg.thread_id);
				if (t) {
					const last_version_id = ranges?.live.items.at(-1)?.version_id ??
						t.last_version_id;
					console.log({ last_version_id });
					api.threads.cache.set(msg.thread_id, {
						...t,
						message_count: t.message_count! - 1,
						last_version_id,
						is_unread: !!t.last_read_id && t.last_read_id < last_version_id,
					});
				}
			});
		} else if (msg.type === "MessageVersionDelete") {
			// TODO
		} else if (msg.type === "RoomMemberUpsert") {
			const m = msg.member;
			const c = room_members.cache.get(m.room_id);
			if (c) {
				c.set(m.user_id, m);
			} else {
				room_members.cache.set(m.room_id, new ReactiveMap());
				room_members.cache.get(m.room_id)!.set(m.user_id, m);
			}
			const l = room_members._cachedListings.get(m.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.user_id === m.user_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, m),
					});
				} else {
					l.mutate({
						...p,
						items: [...p.items, m],
						total: p.total + 1,
					});
				}
			}
		} else if (msg.type === "ThreadMemberUpsert") {
			const m = msg.member;
			const c = thread_members.cache.get(m.thread_id);
			if (c) {
				c.set(m.user_id, m);
			} else {
				thread_members.cache.set(m.thread_id, new ReactiveMap());
				thread_members.cache.get(m.thread_id)!.set(m.user_id, m);
			}
			const l = thread_members._cachedListings.get(m.thread_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.user_id === m.user_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, m),
					});
				} else {
					l.mutate({
						...p,
						items: [...p.items, m],
						total: p.total + 1,
					});
				}
			}
		} else if (msg.type === "RoleCreate") {
			const r = msg.role;
			roles.cache.set(r.id, r);
			const l = roles._cachedListings.get(r.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				l.mutate({
					...p,
					items: [...p.items, r],
					total: p.total + 1,
				});
			}
		} else if (msg.type === "RoleUpdate") {
			const r = msg.role;
			roles.cache.set(r.id, r);
			const l = roles._cachedListings.get(r.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.id === r.id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, r),
					});
				}
			}
		} else if (msg.type === "RoleDelete") {
			roles.cache.delete(msg.role_id);
			const l = roles._cachedListings.get(msg.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.id === msg.role_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1),
						total: p.total - 1,
					});
				}
			}
		} else if (msg.type === "InviteCreate") {
			const { invite } = msg;
			invites.cache.set(invite.code, invite);
			if (invite.target.type === "Room") {
				const room_id = invite.target.room.id;
				const l = invites._cachedListings.get(room_id);
				if (l?.pagination) {
					const p = l.resource.latest;
					if (p) {
						l.mutate({
							...p,
							items: [...p.items, invite],
							total: p.total + 1,
						});
					}
				}
			}
		} else if (msg.type === "InviteDelete") {
			const invite = invites.cache.get(msg.code);
			if (invite) {
				if (invite.target.type === "Room") {
					const room_id = invite.target.room.id;
					const l = invites._cachedListings.get(room_id);
					if (l?.pagination) {
						const p = l.resource.latest;
						if (p) {
							const idx = p.items.findIndex((i) => i.code === invite.code);
							if (idx !== -1) {
								l.mutate({
									...p,
									items: p.items.toSpliced(idx, 1),
								});
							}
						}
					}
				}
			}
			invites.cache.delete(msg.code);
		} else if (msg.type === "BotAdd") {
			// TODO: maybe do something here?
		} else if (msg.type === "ReactionCreate") {
			const { message_id, thread_id, user_id, key } = msg;
			const message = messages.cache.get(message_id);
			if (message) {
				const reactions = message.reactions ?? [];
				const reaction = reactions.find((r) => r.key === key);
				if (reaction) {
					reaction.count++;
					if (user_id === session()?.user_id) {
						reaction.self = true;
					}
				} else {
					reactions.push({
						key,
						count: 1,
						self: user_id === session()?.user_id,
					});
				}
				messages.cache.set(message_id, { ...message, reactions });
			}
		} else if (msg.type === "ReactionDelete") {
			const { message_id, thread_id, user_id, key } = msg;
			const message = messages.cache.get(message_id);
			if (message) {
				const reactions = message.reactions ?? [];
				const reaction = reactions.find((r) => r.key === key);
				if (reaction) {
					reaction.count--;
					if (user_id === session()?.user_id) {
						reaction.self = false;
					}
					if (reaction.count === 0) {
						const idx = reactions.findIndex((r) => r.key === key);
						if (idx !== -1) {
							reactions.splice(idx, 1);
						}
					}
				}
				messages.cache.set(message_id, { ...message, reactions });
			}
		} else if (msg.type === "ReactionPurge") {
			// TODO
		} else if (msg.type === "EmojiCreate") {
			// TODO
		} else if (msg.type === "EmojiDelete") {
			// TODO
		} else if (msg.type === "UserCreate") {
			users.cache.set(msg.user.id, msg.user);
		} else if (msg.type === "UserUpdate") {
			users.cache.set(msg.user.id, msg.user);
			if (msg.user.id === users.cache.get("@self")?.id) {
				users.cache.set("@self", msg.user);
			}
		} else if (msg.type === "UserDelete") {
			users.cache.delete(msg.id);
		} else if (msg.type === "SessionCreate") {
			// NOTE: should this be a SessionUpdate? or a special SessionLogin/SessionAuth event?
			// TODO: dont reload page on auth change
			const s = session();
			if (
				msg.session?.id === s?.id && s.status === "Unauthorized" &&
				msg.session.status === "Authorized"
			) {
				location.reload();
			}
		} else if (msg.type === "SessionUpdate") {
			if (msg.session?.id === session()?.id) {
				setSession(session);
			}
		} else if (msg.type === "SessionDelete") {
			// TODO
		} else if (msg.type === "RelationshipUpsert") {
			// TODO
		} else if (msg.type === "RelationshipDelete") {
			// TODO
		} else if (msg.type === "VoiceState") {
			const state = msg.state as VoiceState | null;
			if (state) {
				voiceStates.set(msg.user_id, state);
			} else {
				voiceStates.delete(msg.user_id);
			}
			if (msg.user_id === users.cache.get("@self")?.id) {
				setVoiceState(state);
			}
		} else if (msg.type === "VoiceDispatch") {
			// handled by rtc.ts
		} else {
			console.warn(`unknown event ${msg.type}`, msg);
		}
	});

	events.on("ready", (msg) => {
		if (msg.user) {
			users.cache.set("@self", msg.user);
			users.cache.set(msg.user.id, msg.user);
		}
		setSession(msg.session);
	});

	async function tempCreateSession() {
		const res = await client.http.POST("/api/v1/session", {
			body: {},
		});
		if (!res.data) {
			console.error("failed to init session", res.response);
			throw new Error("failed to init session");
		}
		const session = res.data;
		localStorage.setItem("token", session.token);
		setSession(session);
		client.start(session.token);
	}

	const api: Api = {
		rooms,
		threads,
		invites,
		roles,
		room_members,
		thread_members,
		users,
		messages,
		media,
		session,
		typing,
		audit_logs,
		tempCreateSession,
		client,
		emoji,
		dms,
		voiceStates,
		voiceState,
		Provider(props: ParentProps) {
			return (
				<ApiContext.Provider value={api}>
					{props.children}
				</ApiContext.Provider>
			);
		},
		events,
	};

	messages.api = api;
	rooms.api = api;
	threads.api = api;
	roles.api = api;
	room_members.api = api;
	thread_members.api = api;
	invites.api = api;
	users.api = api;
	audit_logs.api = api;
	media.api = api;
	emoji.api = api;
	dms.api = api;

	console.log("provider created", api);
	return api;
}

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};

export type Api = {
	rooms: {
		fetch: (room_id?: () => string) => Resource<Room>;
		list: () => Resource<Pagination<Room>>;
		cache: ReactiveMap<string, Room>;
		markRead: (room_id: string) => Promise<void>;
	};
	threads: {
		fetch: (thread_id: () => string) => Resource<Thread>;
		list: (room_id: () => string) => Resource<Pagination<Thread>>;
		cache: ReactiveMap<string, Thread>;
		ack: (
			thread_id: string,
			message_id: string | undefined,
			version_id: string,
		) => Promise<void>;
	};
	dms: {
		list: () => Resource<Pagination<Thread>>;
	};
	invites: {
		fetch: (invite_code: () => string) => Resource<Invite>;
		list: (room_id: () => string) => Resource<Pagination<InviteWithMetadata>>;
		cache: ReactiveMap<string, Invite>;
	};
	roles: {
		fetch: (room_id: () => string, role_id: () => string) => Resource<Role>;
		list: (room_id: () => string) => Resource<Pagination<Role>>;
		cache: ReactiveMap<string, Role>;
	};
	audit_logs: {
		fetch: (room_id: () => string) => Resource<Pagination<AuditLogEntry>>;
	};
	room_members: {
		fetch: (
			room_id: () => string,
			user_id: () => string,
		) => Resource<RoomMember>;
		list: (room_id: () => string) => Resource<Pagination<RoomMember>>;
		cache: ReactiveMap<string, ReactiveMap<string, RoomMember>>;
	};
	thread_members: {
		fetch: (
			thread_id: () => string,
			user_id: () => string,
		) => Resource<ThreadMember>;
		list: (thread_id: () => string) => Resource<Pagination<ThreadMember>>;
		cache: ReactiveMap<string, ReactiveMap<string, ThreadMember>>;
	};
	users: {
		fetch: (user_id: () => string) => Resource<UserWithRelationship>;
		cache: ReactiveMap<string, UserWithRelationship>;
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
	media: {
		fetchInfo: (media_id: () => string) => Resource<Media>;
		cacheInfo: ReactiveMap<string, Media>;
	};
	emoji: {
		fetch: (
			room_id: () => string,
			emoji_id: () => string,
		) => Resource<EmojiCustom>;
		list: (room_id: () => string) => Resource<Pagination<EmojiCustom>>;
		cache: ReactiveMap<string, ReactiveMap<string, EmojiCustom>>;
	};
	session: Accessor<Session | null>;
	typing: ReactiveMap<string, Set<string>>;
	voiceState: Accessor<VoiceState | null>;
	voiceStates: ReactiveMap<string, VoiceState>;
	tempCreateSession: () => void;
	client: Client;
	Provider: Component<ParentProps>;

	events: Emitter<{
		sync: MessageSync;
		ready: MessageReady;
	}>;
};

export type Listing<T> = {
	resource: Resource<Pagination<T>>;
	pagination: Pagination<T> | null;
	mutate: (value: Pagination<T>) => void;
	refetch: () => void;
	prom: Promise<unknown> | null;
};
