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
	InboxListParams,
	Invite,
	InviteWithMetadata,
	Media,
	MemberListGroup,
	Message,
	MessageCreate,
	MessageReady,
	MessageSync,
	Notification,
	Pagination,
	Role,
	Room,
	RoomBan,
	RoomMember,
	Session,
	Thread,
	ThreadMember,
	User,
	UserConfig,
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
import { RoomBans } from "./api/room_bans.ts";
import { Roles } from "./api/roles.ts";
import { AuditLogs } from "./api/audit_log.ts";
import { ThreadMembers } from "./api/thread_members.ts";
import { MediaInfo } from "./api/media.tsx";
import { Emoji } from "./api/emoji.ts";
import { Dms } from "./api/dms.ts";
import { deepEqual } from "./utils/deepEqual.ts";
import { Inbox } from "./api/inbox.ts";

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

export type MemberList = {
	groups: MemberListGroup[];
	items: {
		room_member: RoomMember | null;
		thread_member: ThreadMember | null;
		user: User;
	}[];
};

export function createApi(
	client: Client,
	events: Emitter<{
		sync: MessageSync;
		ready: MessageReady;
	}>,
	{ userConfig, setUserConfig }: {
		userConfig: Accessor<UserConfig>;
		setUserConfig: (c: UserConfig) => void;
	},
) {
	const [session, setSession] = createSignal<Session | null>(null);

	const rooms = new Rooms();
	const threads = new Threads();
	const invites = new Invites();
	const roles = new Roles();
	const room_members = new RoomMembers();
	const room_bans = new RoomBans();
	const thread_members = new ThreadMembers();
	const users = new Users();
	const messages = new Messages();
	const media = new MediaInfo();
	const typing = new ReactiveMap<string, Set<string>>();
	const typing_timeout = new Map<string, Map<string, NodeJS.Timeout>>();
	const audit_logs = new AuditLogs();
	const emoji = new Emoji();
	const dms = new Dms();
	const inbox = new Inbox();
	const voiceStates = new ReactiveMap();
	const [voiceState, setVoiceState] = createSignal();
	const memberLists = new ReactiveMap<string, MemberList>();

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
			const old_thread = threads.cache.get(thread.id);
			threads.cache.set(thread.id, thread);

			if (thread.room_id) {
				const was_archived = !!old_thread?.archived_at;
				const is_archived = !!thread.archived_at;
				const was_removed = !!old_thread?.deleted_at;
				const is_removed = !!thread.deleted_at;

				const get_status = (archived: boolean, removed: boolean) => {
					if (removed) return "removed";
					if (archived) return "archived";
					return "active";
				};

				const old_status = get_status(was_archived, was_removed);
				const new_status = get_status(is_archived, is_removed);

				if (old_status !== new_status) {
					// Thread moved between lists
					const old_listing = (old_status === "active"
						? threads._cachedListings
						: old_status === "archived"
						? (threads as any)._cachedArchivedListings
						: (threads as any)._cachedRemovedListings)?.get(thread.room_id);

					if (old_listing?.pagination) {
						const p = old_listing.pagination;
						const idx = p.items.findIndex((i) =>
							i.id === thread.id
						);
						if (idx !== -1) {
							old_listing.mutate({
								...p,
								items: p.items.toSpliced(idx, 1),
								total: p.total - 1,
							});
						}
					}

					const new_listing = (new_status === "active"
						? threads._cachedListings
						: new_status === "archived"
						? (threads as any)._cachedArchivedListings
						: (threads as any)._cachedRemovedListings)?.get(thread.room_id);

					if (new_listing?.pagination) {
						const p = new_listing.pagination;
						if (p.items.findIndex((i) => i.id === thread.id) === -1) {
							new_listing.mutate({
								...p,
								items: [...p.items, thread],
								total: p.total + 1,
							});
						}
					}
				} else {
					// Thread was updated in place
					const listing = (new_status === "active"
						? threads._cachedListings
						: new_status === "archived"
						? (threads as any)._cachedArchivedListings
						: (threads as any)._cachedRemovedListings)?.get(thread.room_id);

					if (listing?.pagination) {
						const p = listing.pagination;
						const idx = p.items.findIndex((i) =>
							i.id === thread.id
						);
						if (idx !== -1) {
							listing.mutate({
								...p,
								items: p.items.toSpliced(idx, 1, thread),
							});
						}
					}
				}
			}
		} else if (msg.type === "ThreadTyping") {
			const { thread_id, user_id, until } = msg;
			const t = typing.get(thread_id) ?? new Set();
			typing.set(thread_id, new Set([...t, user_id]));

			const timeout = setTimeout(() => {
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
						r.live.items.splice(idx, 1, m);
					} else {
						const id_idx = r.live.items.findIndex((i) => i.id === m.id);
						if (id_idx === -1) {
							r.live.items.push(m);
						}
					}
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
					const tt = typing_timeout.get(m.thread_id)?.get(m.author_id);
					if (tt) clearTimeout(tt);
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
		} else if (msg.type === "MessageDeleteBulk") {
			batch(() => {
				const { thread_id, message_ids } = msg;
				const ranges = messages.cacheRanges.get(thread_id);
				if (ranges) {
					let changed = false;
					for (const message_id of message_ids) {
						messages.cache.delete(message_id);
						const r = ranges.find(message_id);
						if (r) {
							const idx = r.items.findIndex((i) => i.id === message_id);
							if (idx !== -1) {
								r.items.splice(idx, 1);
								changed = true;
							}
						}
					}
					if (changed) {
						messages._updateMutators(ranges, thread_id);
					}
				}

				const t = api.threads.cache.get(thread_id);
				if (t) {
					const last_version_id = ranges?.live.items.at(-1)?.version_id ??
						t.last_version_id;
					api.threads.cache.set(thread_id, {
						...t,
						message_count: t.message_count! - message_ids.length,
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
			const list = memberLists.get(m.room_id);
			if (list) {
				const newItems = list.items.map((item) => {
					if (item.user.id === m.user_id) {
						return { ...item, room_member: m };
					}
					return item;
				});
				memberLists.set(m.room_id, { ...list, items: newItems });
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
			const list = memberLists.get(m.thread_id);
			if (list) {
				const newItems = list.items.map((item) => {
					if (item.user.id === m.user_id) {
						return { ...item, thread_member: m };
					}
					return item;
				});
				memberLists.set(m.thread_id, { ...list, items: newItems });
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
		} else if (msg.type === "RoleReorder") {
			const { room_id, roles: reordered } = msg;
			const l = roles._cachedListings.get(room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const positions = new Map(
					reordered.map((i) => [i.role_id, i.position]),
				);
				const newItems = p.items.map((i) => {
					const pos = positions.get(i.id);
					if (pos) {
						const newRole = { ...i, position: pos };
						roles.cache.set(i.id, newRole);
						return newRole;
					}
					return i;
				});
				newItems.sort((a, b) => b.position - a.position);
				l.mutate({
					...p,
					items: newItems,
				});
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
			} else if (invite.target.type === "Server") {
				const l = invites._cachedServerListing;
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
			invites.cache.delete(msg.code);
			if (msg.target.type === "Room") {
				const room_id = msg.target.room_id;
				const l = invites._cachedListings.get(room_id);
				if (l?.pagination) {
					const p = l.resource.latest;
					if (p) {
						const idx = p.items.findIndex((i) => i.code === msg.code);
						if (idx !== -1) {
							l.mutate({
								...p,
								items: p.items.toSpliced(idx, 1),
								total: p.total - 1,
							});
						}
					}
				}
			} else if (msg.target.type === "Server") {
				const l = invites._cachedServerListing;
				if (l?.pagination) {
					const p = l.resource.latest;
					if (p) {
						const idx = p.items.findIndex((i) => i.code === msg.code);
						if (idx !== -1) {
							l.mutate({
								...p,
								items: p.items.toSpliced(idx, 1),
								total: p.total - 1,
							});
						}
					}
				}
			}
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
			users.cache.set(msg.user.id, {
				...msg.user,
				relationship: {
					note: null,
					relation: null,
					petname: null,
					ignore: null,
				},
			} as UserWithRelationship);
		} else if (msg.type === "UserUpdate") {
			const oldUser = users.cache.get(msg.user.id);
			const updatedUser: UserWithRelationship = {
				...(oldUser ?? {
					relationship: {
						note: null,
						relation: null,
						petname: null,
						ignore: null,
					},
				}),
				...msg.user,
			};
			users.cache.set(msg.user.id, updatedUser);

			if (msg.user.id === users.cache.get("@self")?.id) {
				users.cache.set("@self", updatedUser);
			}

			for (const [id, list] of memberLists.entries()) {
				let wasUpdated = false;
				const newItems = list.items.map((item) => {
					if (item.user.id === msg.user.id) {
						wasUpdated = true;
						return { ...item, user: msg.user };
					}
					return item;
				});

				if (wasUpdated) {
					memberLists.set(id, { ...list, items: newItems });
				}
			}
		} else if (msg.type === "UserConfigGlobal") {
			if (msg.user_id === session()?.user_id) {
				if (!deepEqual(userConfig(), msg.config)) {
					setUserConfig(msg.config);
				}
			}
		} else if (msg.type === "UserConfigRoom") {
			if (msg.user_id === session()?.user_id) {
				const room = rooms.cache.get(msg.room_id);
				if (room) {
					rooms.cache.set(msg.room_id, { ...room, user_config: msg.config });
				}
			}
		} else if (msg.type === "UserConfigThread") {
			if (msg.user_id === session()?.user_id) {
				const thread = threads.cache.get(msg.thread_id);
				if (thread) {
					threads.cache.set(thread.id, {
						...thread,
						user_config: msg.config,
					});
				}
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
			const { user_id, relationship } = msg;
			const user = users.cache.get(user_id);
			if (user) {
				users.cache.set(user_id, { ...user, relationship });
			}
		} else if (msg.type === "RelationshipDelete") {
			const { user_id } = msg;
			const user = users.cache.get(user_id);
			if (user) {
				users.cache.set(user_id, {
					...user,
					relationship: {
						note: null,
						relation: null,
						petname: null,
						ignore: null,
					},
				});
			}
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
		} else if (msg.type === "AuditLogEntryCreate") {
			const cached = audit_logs._cachedListings.get(msg.entry.room_id);
			if (cached?.pagination) {
				cached.pagination.items.unshift(msg.entry);
				cached.pagination.total += 1;
			}
		} else if (msg.type === "BanCreate") {
			const { ban } = msg;
			const c = room_bans.cache.get(ban.room_id);
			if (c) {
				c.set(ban.user_id, ban);
			} else {
				room_bans.cache.set(ban.room_id, new ReactiveMap());
				room_bans.cache.get(ban.room_id)!.set(ban.user_id, ban);
			}
			const l = room_bans._cachedListings.get(ban.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.user_id === ban.user_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, ban),
					});
				} else {
					l.mutate({
						...p,
						items: [...p.items, ban],
						total: p.total + 1,
					});
				}
			}
		} else if (msg.type === "BanDelete") {
			const { room_id, user_id } = msg;
			const c = room_bans.cache.get(room_id);
			if (c) {
				c.delete(user_id);
			}
			const l = room_bans._cachedListings.get(room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.user_id === user_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1),
						total: p.total - 1,
					});
				}
			}
		} else if (msg.type === "MemberListSync") {
			const { room_id, thread_id, ops, groups } = msg;
			const id = room_id ?? thread_id;
			if (!id) return;

			let list = memberLists.get(id);
			if (!list) {
				list = { groups: [], items: [] };
				memberLists.set(id, list);
			}

			for (const op of ops) {
				if (op.type === "Sync") {
					const items = op.users.map((user, i) => ({
						user,
						room_member: op.room_members?.[i] ?? null,
						thread_member: op.thread_members?.[i] ?? null,
					}));
					list.items.splice(op.position, items.length, ...items);
				} else if (op.type === "Insert") {
					const item = {
						user: op.user,
						room_member: op.room_member ?? null,
						thread_member: op.thread_member ?? null,
					};
					list.items.splice(op.position, 0, item);
				} else if (op.type === "Delete") {
					list.items.splice(op.position, op.count);
				}
			}
			list.groups = groups;
			memberLists.set(id, { ...list });
		} else if (msg.type === "InboxNotificationCreate") {
			if (msg.user_id === session()?.user_id) {
				inbox.cache.set(msg.notification.id, msg.notification);
				for (const listing of inbox._listings.values()) {
					listing.refetch();
				}
			}
		} else if (msg.type === "InboxMarkRead") {
			if (msg.user_id === session()?.user_id) {
				for (const listing of inbox._listings.values()) {
					listing.refetch();
				}
			}
		} else if (msg.type === "InboxMarkUnread") {
			if (msg.user_id === session()?.user_id) {
				for (const listing of inbox._listings.values()) {
					listing.refetch();
				}
			}
		} else if (msg.type === "InboxFlush") {
			if (msg.user_id === session()?.user_id) {
				for (const listing of inbox._listings.values()) {
					listing.refetch();
				}
			}
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
		room_bans,
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
		inbox,
		voiceStates,
		voiceState,
		memberLists,
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
	room_bans.api = api;
	thread_members.api = api;
	invites.api = api;
	users.api = api;
	audit_logs.api = api;
	media.api = api;
	emoji.api = api;
	dms.api = api;
	inbox.api = api;

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
		list_all: () => Resource<Pagination<Room>>;
		cache: ReactiveMap<string, Room>;
		markRead: (room_id: string) => Promise<void>;
	};
	threads: {
		fetch: (thread_id: () => string) => Resource<Thread>;
		list: (room_id: () => string) => Resource<Pagination<Thread>>;
		listArchived: (room_id: () => string) => Resource<Pagination<Thread>>;
		listRemoved: (room_id: () => string) => Resource<Pagination<Thread>>;
		cache: ReactiveMap<string, Thread>;
		ack: (
			thread_id: string,
			message_id: string | undefined,
			version_id: string,
		) => Promise<void>;
		lock: (thread_id: string) => Promise<void>;
		unlock: (thread_id: string) => Promise<void>;
	};
	dms: {
		list: () => Resource<Pagination<Thread>>;
	};
	inbox: Inbox;
	invites: {
		fetch: (invite_code: () => string) => Resource<Invite>;
		list: (room_id: () => string) => Resource<Pagination<InviteWithMetadata>>;
		list_server: () => Resource<Pagination<InviteWithMetadata>>;
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
		subscribeList: (room_id: string, ranges: [number, number][]) => void;
	};
	room_bans: {
		fetch: (
			room_id: () => string,
			user_id: () => string,
		) => Resource<RoomBan>;
		list: (room_id: () => string) => Resource<Pagination<RoomBan>>;
		cache: ReactiveMap<string, ReactiveMap<string, RoomBan>>;
	};
	thread_members: {
		fetch: (
			thread_id: () => string,
			user_id: () => string,
		) => Resource<ThreadMember>;
		list: (thread_id: () => string) => Resource<Pagination<ThreadMember>>;
		cache: ReactiveMap<string, ReactiveMap<string, ThreadMember>>;
		subscribeList: (thread_id: string, ranges: [number, number][]) => void;
	};
	users: {
		fetch: (user_id: () => string) => Resource<UserWithRelationship>;
		list: () => Resource<Pagination<User>>;
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
		listPinned: (
			thread_id: () => string,
		) => Resource<Pagination<Message>>;
		fetch: (
			thread_id: () => string,
			message_id: () => string,
		) => Resource<Message>;
		cache: ReactiveMap<string, Message>;
		cacheRanges: Map<string, MessageRanges>;
		edit: (
			thread_id: string,
			message_id: string,
			content: string,
		) => Promise<Message>;
		pin: (thread_id: string, message_id: string) => Promise<void>;
		unpin: (thread_id: string, message_id: string) => Promise<void>;
		reorderPins: (
			thread_id: string,
			messages: { id: string; position: number }[],
		) => Promise<void>;
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
	memberLists: ReactiveMap<string, MemberList>;
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
