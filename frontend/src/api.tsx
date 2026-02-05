// TODO: this file is getting big and should probably be split and refactored
// i'm copypasting stuff for now, but will refactor out abstractions later

// TODO: also, the architecture with solidjs resources feels very... bad? to work with?
// but refactoring everything would be a pain and im not sure how i could improve this code

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
	Client,
	ClientState,
	Media,
	MemberListGroup,
	Message,
	MessageCreate,
	MessageEnvelope,
	MessageReady,
	MessageSync,
	Pagination,
	RoomMember,
	Session,
	ThreadMember,
	User,
	UserConfig,
	UserWithRelationship,
	VoiceState,
} from "sdk";
import type { Emitter } from "@solid-primitives/event-bus";
import { Messages } from "./api/messages.ts";
import { Rooms } from "./api/rooms.ts";
import { Channels } from "./api/channels.ts";
import { Threads } from "./api/threads.ts";
import { Users } from "./api/users.ts";
import { Invites } from "./api/invite.ts";
import { ChannelInvites } from "./api/channel_invite.ts";
import { Webhooks } from "./api/webhooks.ts";
import { RoomMembers } from "./api/room_members.ts";
import { RoomBans } from "./api/room_bans.ts";
import { Roles } from "./api/roles.ts";
import { AuditLogs } from "./api/audit_log.ts";
import { ThreadMembers } from "./api/thread_members.ts";
import { MediaInfo } from "./api/media.tsx";
import { Emoji } from "./api/emoji.ts";
import { Reactions } from "./api/reactions.ts";
import { Dms } from "./api/dms.ts";
import { Auth } from "./api/auth.ts";
import { Sessions } from "./api/sessions.ts";
import { notificationPermission } from "./notification.ts";
import { deepEqual } from "./utils/deepEqual.ts";
import { Inbox } from "./api/inbox.ts";
import { generateNotificationIcon } from "./drawing.ts";

export type Json =
	| number
	| string
	| boolean
	| Array<Json>
	| { [k in string]: Json };

type MessageV2 = {
	id: string;
	channel_id: string;
	latest_version: {
		version_id: string;
		author_id?: string;
		[K: string]: any;
	};
	pinned?: { time: string; position: number };
	reactions?: any[];
	deleted_at?: string;
	removed_at?: string;
	created_at: string;
	author_id: string;
	thread?: any;
	[K: string]: any;
};

function convertV2MessageToV1(message: MessageV2): Message {
	return {
		...message.latest_version,
		id: message.id,
		channel_id: message.channel_id,
		version_id: message.latest_version.version_id,
		nonce: message.nonce ?? null,
		author_id: message.author_id,
		pinned: message.pinned,
		reactions: message.reactions,
		created_at: message.created_at,
		deleted_at: message.deleted_at,
		removed_at: message.removed_at,
		edited_at: message.latest_version.version_id !== message.id
			? message.latest_version.created_at
			: null,
		thread: message.thread,
	};
}

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
		sync: [MessageSync, MessageEnvelope];
		ready: MessageReady;
	}>,
	{ userConfig, setUserConfig }: {
		userConfig: Accessor<UserConfig>;
		setUserConfig: (c: UserConfig) => void;
	},
) {
	const [session, setSession] = createSignal<Session | null>(null);
	const [clientState, setClientState] = createSignal<ClientState>("stopped");
	client.state.subscribe(setClientState);

	const rooms = new Rooms();
	const channels = new Channels();
	const threads = new Threads();
	const invites = new Invites();
	const channel_invites = new ChannelInvites();
	const webhooks = new Webhooks();
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
	const reactions = new Reactions();
	const dms = new Dms();
	const auth = new Auth();
	const sessions = new Sessions();
	const inbox = new Inbox();
	const voiceStates = new ReactiveMap();
	const [voiceState, setVoiceState] = createSignal();

	events.on("sync", ([msg, raw]) => {
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
		} else if (msg.type === "ChannelCreate") {
			const { channel } = msg;
			channels.cache.set(channel.id, channel);
			if (channel.room_id) {
				const l = threads._cachedListings.get(channel.room_id);
				if (l?.pagination) {
					const p = l.pagination;
					for (const mut of threads._listingMutators) {
						if (mut.room_id === channel.room_id) {
							mut.mutate({
								...p,
								items: [...p.items, channel],
								total: p.total + 1,
							});
						}
					}
				}
			}
		} else if (msg.type === "ChannelUpdate") {
			const { channel: thread } = msg;
			const old_thread = channels.cache.get(thread.id);
			channels.cache.set(thread.id, thread);

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
		} else if (msg.type === "ChannelTyping") {
			const { channel_id: thread_id, user_id, until } = msg;
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
		} else if (msg.type === "ChannelAck") {
			// TODO
		} else if (msg.type === "MessageCreate") {
			const m = "latest_version" in msg.message
				? convertV2MessageToV1(msg.message)
				: msg.message;
			m.nonce = raw.nonce;

			const me = users.cache.get("@self");
			if (
				me && m.author_id !== me.id && m.type === "DefaultMarkdown" &&
				notificationPermission() === "granted" &&
				userConfig().frontend["desktop_notifs"] === "yes"
			) {
				const mentions = m.mentions;
				let is_mentioned = false;
				if (mentions) {
					if (mentions.users.some((u) => u.id === me.id)) {
						is_mentioned = true;
					}
					if (!is_mentioned && mentions.everyone) {
						is_mentioned = true;
					}
					if (!is_mentioned && mentions.roles.length > 0) {
						const channel = channels.cache.get(m.channel_id);
						if (channel?.room_id) {
							const room_member = room_members.cache.get(channel.room_id)?.get(
								me.id,
							);
							if (room_member) {
								for (const role of mentions.roles) {
									if (room_member.roles.some((r) => r.id === role.id)) {
										is_mentioned = true;
										break;
									}
								}
							}
						}
					}
				}

				// Check notification settings for mentions
				if (is_mentioned && userConfig().notifs.mentions === "Notify") {
					const author = users.cache.get(m.author_id);
					const channel = channels.cache.get(m.channel_id);
					const title = `${author?.name ?? "Someone"} in #${
						channel?.name ?? "channel"
					}`;
					const rawContent = m.content ?? "";
					const processedContent = stripMarkdownAndResolveMentions(
						rawContent,
						m.channel_id,
					);
					const body = processedContent.substring(0, 200);

					(async () => {
						let icon: string | undefined;
						if (author) {
							const room = channel?.room_id
								? rooms.cache.get(channel.room_id)
								: undefined;
							const iconBlob = await generateNotificationIcon(
								author,
								room ?? undefined,
							);
							if (iconBlob) {
								icon = URL.createObjectURL(iconBlob);
							}
						}

						const notification = new Notification(title, { body, icon });
						notification.onclick = () => {
							window.focus();
							location.href = `/channel/${m.channel_id}/message/${m.id}`;
						};
						if (icon) {
							notification.onclose = () => {
								URL.revokeObjectURL(icon!);
							};
						}
					})();
				}
			}

			const r = messages.cacheRanges.get(m.channel_id);
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
					messages._updateMutators(r, m.channel_id);
				});
			}

			const t = api.channels.cache.get(m.channel_id);
			if (t) {
				api.channels.cache.set(m.channel_id, {
					...t,
					message_count: (t.message_count ?? 0) + (is_new ? 1 : 0),
					last_version_id: m.version_id,
					is_unread,
				});
			}

			{
				const t = typing.get(m.channel_id);
				if (t) {
					t.delete(m.author_id);
					typing.set(m.channel_id, new Set(t));
					const tt = typing_timeout.get(m.channel_id)?.get(m.author_id);
					if (tt) clearTimeout(tt);
				}
			}

			for (
				const att of m.type === "DefaultMarkdown" ? m.attachments ?? [] : []
			) {
				media.cacheInfo.set(att.id, att);
			}
		} else if (msg.type === "MessageUpdate") {
			const m = "latest_version" in msg.message
				? convertV2MessageToV1(msg.message)
				: msg.message;
			const r = messages.cacheRanges.get(m.channel_id);
			if (r) {
				const idx = r.live.items.findIndex((i) => i.id === m.id);
				if (idx !== -1) {
					console.log("Message Update edit");
					r.live.items.splice(idx, 1, m);
				}
				batch(() => {
					messages.cache.set(m.id, m);
					messages._updateMutators(r, m.channel_id);
				});
			}
		} else if (msg.type === "MessageDelete") {
			batch(() => {
				const { message_id, channel_id: thread_id } = msg;
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
				const t = api.channels.cache.get(msg.channel_id);
				if (t) {
					const last_version_id = ranges?.live.items.at(-1)?.version_id ??
						t.last_version_id;
					console.log({ last_version_id });
					api.channels.cache.set(msg.channel_id, {
						...t,
						message_count: t.message_count! - 1,
						last_version_id,
						is_unread: !!t.last_read_id && t.last_read_id < last_version_id,
					});
				}
			});
		} else if (msg.type === "MessageDeleteBulk") {
			batch(() => {
				const { channel_id: thread_id, message_ids } = msg;
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

				const t = api.channels.cache.get(thread_id);
				if (t) {
					const last_version_id = ranges?.live.items.at(-1)?.version_id ??
						t.last_version_id;
					api.channels.cache.set(thread_id, {
						...t,
						message_count: t.message_count! - message_ids.length,
						last_version_id,
						is_unread: !!t.last_read_id && t.last_read_id < last_version_id,
					});
				}
			});
		} else if (msg.type === "MessageVersionDelete") {
			// TODO
		} else if (
			msg.type === "RoomMemberCreate" || msg.type === "RoomMemberUpdate"
		) {
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
		} else if (msg.type === "RoomMemberDelete") {
			const { room_id, user_id } = msg;
			const c = room_members.cache.get(room_id);
			if (c) {
				c.delete(user_id);
			}
			const l = room_members._cachedListings.get(room_id);
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
		} else if (msg.type === "ThreadMemberUpsert") {
			const { thread_id, added, removed } = msg;

			for (const member of added) {
				const c = thread_members.cache.get(thread_id);
				if (c) {
					c.set(member.user_id, member);
				} else {
					thread_members.cache.set(thread_id, new ReactiveMap());
					thread_members.cache.get(thread_id)!.set(member.user_id, member);
				}
				const l = thread_members._cachedListings.get(thread_id);
				if (l?.resource.latest) {
					const p = l.resource.latest;
					const idx = p.items.findIndex((i) => i.user_id === member.user_id);
					if (idx !== -1) {
						l.mutate({
							...p,
							items: p.items.toSpliced(idx, 1, member),
						});
					} else {
						l.mutate({
							...p,
							items: [...p.items, member],
							total: p.total + 1,
						});
					}
				}
			}

			for (const user_id of removed) {
				const c = thread_members.cache.get(thread_id);
				if (c) {
					c.delete(user_id);
				}
				const l = thread_members._cachedListings.get(thread_id);
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
			} else if (invite.target.type === "Gdm") {
				const channel_id = invite.target.channel.id;
				const l = channel_invites._cachedListings.get(channel_id);
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
		} else if (msg.type === "RatelimitUpdate") {
			const { channel_id, slowmode_message_expire_at } = msg;
			const [_ch, chUpdate] = api.ctx.channel_contexts.get(channel_id)!;
			if (slowmode_message_expire_at) {
				const expireDate = new Date(slowmode_message_expire_at);
				chUpdate("slowmode_expire_at", expireDate);
			} else {
				chUpdate("slowmode_expire_at", undefined);
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
			} else if (msg.target.type === "Gdm") {
				const channel_id = msg.target.channel_id;
				const l = channel_invites._cachedListings.get(channel_id);
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
			const { message_id, channel_id, user_id, key } = msg;
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
			const { message_id, channel_id, user_id, key } = msg;
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
		} else if (msg.type === "ReactionDeleteKey") {
			const { message_id, key } = msg;
			const message = messages.cache.get(message_id);
			if (message) {
				const reactions = message.reactions ?? [];
				const idx = reactions.findIndex((r) => r.key === key);
				if (idx !== -1) {
					reactions.splice(idx, 1);
				}
				messages.cache.set(message_id, { ...message, reactions });
			}
		} else if (msg.type === "ReactionDeleteAll") {
			const { message_id } = msg;
			const message = messages.cache.get(message_id);
			if (message) {
				messages.cache.set(message_id, { ...message, reactions: [] });
			}
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
		} else if (msg.type === "PresenceUpdate") {
			const { user_id, presence } = msg;
			const user = users.cache.get(user_id);
			if (user) {
				const newUser = { ...user, presence };
				users.cache.set(user_id, newUser);

				if (user_id === users.cache.get("@self")?.id) {
					users.cache.set("@self", newUser);
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
		} else if (msg.type === "UserConfigChannel") {
			if (msg.user_id === session()?.user_id) {
				const thread = channels.cache.get(msg.channel_id);
				if (thread) {
					channels.cache.set(thread.id, {
						...thread,
						user_config: msg.config,
					});
				}
			}
		} else if (msg.type === "UserConfigUser") {
			if (msg.user_id === session()?.user_id) {
				const user = users.cache.get(msg.target_user_id);
				if (user) {
					users.cache.set(msg.target_user_id, {
						...user,
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
			const { target_user_id, relationship } = msg;
			const user = users.cache.get(target_user_id);
			if (user) {
				users.cache.set(target_user_id, { ...user, relationship });
			}
		} else if (msg.type === "RelationshipDelete") {
			const { target_user_id } = msg;
			const user = users.cache.get(target_user_id);
			if (user) {
				users.cache.set(target_user_id, {
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
		} else if (msg.type === "WebhookCreate") {
			const { webhook } = msg;
			webhooks.cache.set(webhook.id, webhook);
			const l = webhooks._cachedListings.get(webhook.channel_id);
			if (l?.pagination) {
				const p = l.resource.latest;
				if (p) {
					l.mutate({
						...p,
						items: [...p.items, webhook],
						total: p.total + 1,
					});
				}
			}
		} else if (msg.type === "WebhookUpdate") {
			const { webhook } = msg;
			webhooks.cache.set(webhook.id, webhook);
			const l = webhooks._cachedListings.get(webhook.channel_id);
			if (l?.pagination) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.id === webhook.id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, webhook),
					});
				}
			}
		} else if (msg.type === "WebhookDelete") {
			webhooks.cache.delete(msg.webhook_id);
			const l = webhooks._cachedListings.get(msg.channel_id);
			if (l?.pagination) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.id === msg.webhook_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1),
						total: p.total - 1,
					});
				}
			}
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
			// console.warn(`unknown event ${msg.type}`, msg);
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

	const stripMarkdownAndResolveMentions = (
		content: string,
		thread_id: string,
	) => {
		let processedContent = content;

		// Replace user mentions <@user-id> with user names
		const userMentionRegex =
			/<@([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
		processedContent = processedContent.replace(
			userMentionRegex,
			(match, userId) => {
				const user = users.cache.get(userId);
				return user ? `@${user.name}` : match; // Keep original if user not found
			},
		);

		// Replace channel mentions <#channel-id> with channel names
		const channelMentionRegex =
			/<#([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
		processedContent = processedContent.replace(
			channelMentionRegex,
			(match, channelId) => {
				const channel = channels.cache.get(channelId);
				return channel ? `#${channel.name}` : match; // Keep original if channel not found
			},
		);

		// Replace role mentions <@&role-id> with role names
		const roleMentionRegex =
			/<@&([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})>/g;
		processedContent = processedContent.replace(
			roleMentionRegex,
			(match, roleId) => {
				const thread = channels.cache.get(thread_id);
				if (!thread?.room_id) return match; // Need room_id to get role
				const role = roles.cache.get(roleId);
				return role ? `@${role.name}` : match; // Keep original if role not found
			},
		);

		// Replace emoji mentions <:name:id> with emoji name
		const emojiMentionRegex =
			/<:(\w+):[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}>/g;
		processedContent = processedContent.replace(
			emojiMentionRegex,
			(match, emojiName) => {
				return `:${emojiName}:`;
			},
		);

		// Remove basic markdown formatting
		// Bold: **text** -> text
		processedContent = processedContent.replace(/\*\*(.*?)\*\*/g, "$1");
		// Italic: *text* or _text_ -> text
		processedContent = processedContent.replace(/([*_])(.*?)\1/g, "$2");
		// Strikethrough: ~~text~~ -> text
		processedContent = processedContent.replace(/~~(.*?)~~/g, "$1");
		// Code: `text` -> text
		processedContent = processedContent.replace(/`(.*?)`/g, "$1");
		// Code blocks: ```language\ntext\n``` -> text
		processedContent = processedContent.replace(
			/```(?:\w+\n)?\n?([\s\S]*?)\n?```/g,
			"$1",
		);
		// Blockquotes: > text on new lines -> text
		processedContent = processedContent.replace(/^ *>(.*)$/gm, "$1");
		// Links: [text](url) -> text
		processedContent = processedContent.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");

		// Clean up extra whitespace
		processedContent = processedContent.replace(/\s+/g, " ").trim();

		return processedContent;
	};

	const api: Api = {
		rooms,
		channels,
		threads,
		invites,
		channel_invites,
		webhooks,
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
		clientState,
		emoji,
		reactions,
		dms,
		auth,
		sessions,
		inbox,
		voiceStates,
		voiceState,
		stripMarkdownAndResolveMentions,
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
	channels.api = api;
	threads.api = api;
	roles.api = api;
	room_members.api = api;
	room_bans.api = api;
	thread_members.api = api;
	invites.api = api;
	channel_invites.api = api;
	webhooks.api = api;
	users.api = api;
	audit_logs.api = api;
	media.api = api;
	emoji.api = api;
	reactions.api = api;
	dms.api = api;
	auth.api = api;
	sessions.api = api;
	inbox.api = api;

	console.log("provider created", api);
	return api;
}

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};

export type Api = {
	rooms: Rooms;
	channels: Channels;
	threads: Threads;
	dms: Dms;
	auth: Auth;
	sessions: Sessions;
	inbox: Inbox;
	invites: Invites;
	channel_invites: ChannelInvites;
	webhooks: Webhooks;
	roles: Roles;
	audit_logs: AuditLogs;
	room_members: RoomMembers;
	room_bans: RoomBans;
	thread_members: ThreadMembers;
	users: Users;
	messages: Messages;
	media: Media;
	emoji: Emoji;
	reactions: Reactions;
	session: Accessor<Session | null>;
	typing: ReactiveMap<string, Set<string>>;
	voiceState: Accessor<VoiceState | null>;
	voiceStates: ReactiveMap<string, VoiceState>;
	tempCreateSession: () => void;
	client: Client;
	clientState: Accessor<ClientState>;
	Provider: Component<ParentProps>;

	events: Emitter<{
		sync: [MessageSync, MessageEnvelope];
		ready: MessageReady;
	}>;

	// Utilities
	stripMarkdownAndResolveMentions: (
		content: string,
		thread_id: string,
	) => string;
};

export type Listing<T> = {
	resource: Resource<Pagination<T>>;
	pagination: Pagination<T> | null;
	mutate: (value: Pagination<T>) => void;
	refetch: () => void;
	prom: Promise<unknown> | null;
};
