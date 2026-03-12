// TODO: this file is getting big and should probably be split and refactored
// i'm copypasting stuff for now, but will refactor out abstractions later

// TODO: also, the architecture with solidjs resources feels very... bad? to work with?
// but refactoring everything would be a pain and im not sure how i could improve this code

import {
	type Accessor,
	batch,
	type Component,
	createContext,
	createEffect,
	createSignal,
	type ParentProps,
	type Resource,
	useContext,
} from "solid-js";
import { ReactiveMap } from "@solid-primitives/map";
import type {
	Channel,
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
	Preferences,
	Role,
	Room,
	RoomMember,
	Session,
	ThreadMember,
	User,
	UserWithRelationship,
	VoiceState,
} from "sdk";
import type { Emitter } from "@solid-primitives/event-bus";
import { Rooms } from "./api/rooms.ts";
import { Channels } from "./api/channels.ts";
import { Threads } from "./api/threads.ts";
import { Users } from "./api/users.ts";
import { Invites } from "./api/invite.ts";
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
import { Tags } from "./api/tags.ts";
import { notificationPermission } from "./notification.ts";
import {
	stripMarkdownAndResolveMentions as stripMarkdownAndResolveMentionsOriginal,
} from "./notification-util.ts";
import { deepEqual } from "./utils/deepEqual.ts";
import { Inbox } from "./api/inbox.ts";
import { Push } from "./api/push.ts";
import { DocumentsService } from "./api/services/DocumentsService.ts";
import { RoomAnalytics } from "./api/room_analytics.ts";
import { generateNotificationIcon } from "./drawing.ts";

import { RootStore } from "./api/core/Store.ts";

export type Json =
	| number
	| string
	| boolean
	| Array<Json>
	| { [k in string]: Json };

const ApiContext = createContext<Api>();
export const RootStoreContext = createContext<RootStore>();

export function useApi() {
	return useContext(ApiContext)!;
}

export function useApi2() {
	return useContext(RootStoreContext)!;
}

export function useRooms2() {
	return useApi2().rooms;
}

export function useChannels2() {
	return useApi2().channels;
}

export function useUsers2() {
	return useApi2().users;
}

export function useRoles2() {
	return useApi2().roles;
}

export function useSessions2() {
	return useApi2().sessions;
}

export function useMessages2() {
	return useApi2().messages;
}

export function useRoomMembers2() {
	return useApi2().roomMembers;
}

export function useThreadMembers2() {
	return useApi2().threadMembers;
}

export function useMemberList2() {
	return useApi2().memberLists;
}

export type MemberList = {
	groups: MemberListGroup[];
	items: {
		room_member: RoomMember | null;
		thread_member: ThreadMember | null;
		user: User;
	}[];
};

function updateSWState(
	apiUrl: string,
	token: string | null,
	sessionId?: string | null,
) {
	const request = indexedDB.open("sw-state", 1);
	request.onupgradeneeded = () => {
		const db = request.result;
		db.createObjectStore("state");
	};
	request.onsuccess = () => {
		const db = request.result;
		const tx = db.transaction("state", "readwrite");
		const store = tx.objectStore("state");
		store.put(apiUrl, "api_url");
		store.put(token, "token");
		if (sessionId && token) {
			store.put(token, `token:${sessionId}`);
		}
	};
}

const areReactionKeysEqual = (a: any, b: any): boolean => {
	if (a.type !== b.type) return false;
	if (a.type === "Text") return a.content === b.content;
	if (a.type === "Custom") return a.id === b.id;
	return false;
};

export function createApi(
	client: Client,
	events: Emitter<{
		sync: [MessageSync, MessageEnvelope];
		ready: MessageReady;
	}>,
	{ preferences, setPreferences, store }: {
		preferences: Accessor<Preferences>;
		setPreferences: (c: Preferences) => void;
		store: RootStore;
	},
) {
	const [session, setSession] = createSignal<Session | null>(null);
	const [preferencesLoaded, setPreferencesLoaded] = createSignal(false);
	const [clientState, setClientState] = createSignal<ClientState>("stopped");
	client.state.subscribe(setClientState);

	createEffect(() => {
		const s = session();
		updateSWState(client.opts.apiUrl, client.opts.token ?? null, s?.id);
	});

	const rooms = new Rooms();
	const channels = new Channels();
	const threads = new Threads();
	const invites = new Invites();
	const webhooks = new Webhooks();
	const roles = new Roles();
	const room_members = new RoomMembers();
	const room_bans = new RoomBans();
	const thread_members = new ThreadMembers();
	const users = new Users();
	const media = new MediaInfo();
	const typing = new ReactiveMap<string, Set<string>>();
	const typing_timeout = new Map<string, Map<string, NodeJS.Timeout>>();
	const audit_logs = new AuditLogs();
	const emoji = new Emoji();
	const reactions = new Reactions();
	const dms = new Dms();
	const auth = new Auth();
	const sessions = new Sessions();
	const push = new Push();
	const inbox = new Inbox();
	const tags = new Tags();
	const documents = new DocumentsService(store);
	const room_analytics = new RoomAnalytics();
	const voiceStates = new ReactiveMap<string, VoiceState>();
	const [voiceState, setVoiceState] = createSignal<VoiceState | null>(null);

	// Helper to safely get current user ID from session
	const getCurrentUserId = (): string | undefined => {
		const s = session();
		if (s && s.status !== "Unauthorized") {
			return s.user_id;
		}
		return undefined;
	};

	events.on("sync", async ([msg, raw]) => {
		if (msg.type === "Ambient") {
			batch(() => {
				console.time("process ambient");

				for (const room of msg.rooms) {
					rooms.cache.set(room.id, room);
				}

				for (const channel of msg.channels) {
					channels.normalize(channel);
					channels.cache.set(channel.id, channel);
				}

				for (const thread of msg.threads) {
					channels.normalize(thread);
					channels.cache.set(thread.id, thread);
				}

				for (const role of msg.roles) {
					roles.cache.set(role.id, role);
				}

				for (const member of msg.room_members) {
					room_members.upsert(member);
				}

				rooms._cachedListing.mutate({
					items: msg.rooms,
					total: msg.rooms.length,
					has_more: false,
					cursor: null,
				});

				const channelsByRoom = new Map<string, Channel[]>();
				const threadsByRoom = new Map<string, Channel[]>();
				for (const channel of msg.channels) {
					if (channel.room_id) {
						const arr = channelsByRoom.get(channel.room_id) ?? [];
						arr.push(channel);
						channelsByRoom.set(channel.room_id, arr);
					}
				}

				for (const thread of msg.threads) {
					if (thread.room_id) {
						const arr = threadsByRoom.get(thread.room_id) ?? [];
						arr.push(thread);
						threadsByRoom.set(thread.room_id, arr);
					}
				}

				for (const [room_id, channelList] of channelsByRoom) {
					channels._getOrCreateListing(channels._cachedListings, room_id)
						.mutate({
							items: channelList,
							total: channelList.length,
							has_more: false,
							cursor: null,
						});
				}

				for (const [room_id, threadList] of threadsByRoom) {
					channels._getOrCreateListing(
						channels._cachedListingsArchived,
						room_id,
					).mutate({
						items: threadList,
						total: threadList.length,
						has_more: false,
						cursor: null,
					});
				}

				const rolesByRoom = new Map<string, Role[]>();
				for (const role of msg.roles) {
					const arr = rolesByRoom.get(role.room_id) ?? [];
					arr.push(role);
					rolesByRoom.set(role.room_id, arr);
				}

				for (const [room_id, roleList] of rolesByRoom) {
					roles._getOrCreateListing(roles._cachedListings, room_id).mutate({
						items: roleList,
						total: roleList.length,
						has_more: false,
						cursor: null,
					});
				}

				const membersByRoom = new Map<string, RoomMember[]>();
				for (const member of msg.room_members) {
					const arr = membersByRoom.get(member.room_id) ?? [];
					arr.push(member);
					membersByRoom.set(member.room_id, arr);
				}

				for (const [room_id, memberList] of membersByRoom) {
					room_members._getOrCreateListing(room_id).mutate({
						items: memberList,
						total: memberList.length,
						has_more: false,
						cursor: null,
					});
				}

				setPreferences(msg.config);
				setPreferencesLoaded(true);

				console.timeEnd("process ambient");
			});
		} else if (msg.type === "RoomCreate" || msg.type === "RoomUpdate") {
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
						items: [...p.items.slice(0, idx), room, ...p.items.slice(idx + 1)],
					});
				}
			}
		} else if (msg.type === "ChannelCreate") {
			const { channel } = msg;
			api.channels.normalize(channel);
			channels.cache.set(channel.id, channel);
			if (channel.room_id) {
				const l = channels._cachedListings.get(channel.room_id);
				if (l?.pagination) {
					const p = l.pagination;
					for (const mut of channels._listingMutators) {
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
			api.channels.normalize(thread);
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
						? channels._cachedListings
						: old_status === "archived"
						? channels._cachedListingsArchived
						: channels._cachedListingsRemoved).get(thread.room_id);

					if (old_listing?.pagination) {
						const p = old_listing.pagination;
						const idx = p.items.findIndex((i) =>
							i.id === thread.id
						);
						if (idx !== -1) {
							old_listing.mutate({
								...p,
								items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
								total: p.total - 1,
							});
						}
					}

					const new_listing = (new_status === "active"
						? channels._cachedListings
						: new_status === "archived"
						? channels._cachedListingsArchived
						: channels._cachedListingsRemoved).get(thread.room_id);

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
						? channels._cachedListings
						: new_status === "archived"
						? channels._cachedListingsArchived
						: channels._cachedListingsRemoved).get(thread.room_id);

					if (listing?.pagination) {
						const p = listing.pagination;
						const idx = p.items.findIndex((i) =>
							i.id === thread.id
						);
						if (idx !== -1) {
							listing.mutate({
								...p,
								items: [
									...p.items.slice(0, idx),
									thread,
									...p.items.slice(idx + 1),
								],
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
			const { channel_id, version_id } = msg;
			const t = channels.cache.get(channel_id);
			if (t) {
				const is_unread = version_id < (t.last_version_id ?? "");
				if (
					t.last_read_id === version_id &&
					t.mention_count === 0 &&
					t.is_unread === is_unread
				) {
					return;
				}
				channels.cache.set(channel_id, {
					...t,
					last_read_id: version_id,
					mention_count: 0,
					is_unread,
				});
			}
		} else if (msg.type === "MessageCreate") {
			const m = msg.message as Message;
			// Only set nonce if raw is a Sync message
			if (raw.op === "Sync") {
				m.nonce = raw.nonce;
			}

			const me = users.cache.get("@self");
			let is_mentioned = false;
			const mentions = m.latest_version.mentions;
			if (
				me && m.author_id !== me.id &&
				m.latest_version.type === "DefaultMarkdown" && mentions
			) {
				if (mentions.users?.some((u) => u.id === me.id)) {
					is_mentioned = true;
				}
				if (!is_mentioned && mentions.everyone) {
					is_mentioned = true;
				}
				if (!is_mentioned && mentions.roles && mentions.roles.length > 0) {
					const channel = channels.cache.get(m.channel_id);
					if (channel?.room_id) {
						const room_member = room_members.cache.get(channel.room_id)?.get(
							me.id,
						);
						if (room_member && mentions.roles) {
							for (const role of mentions.roles) {
								if (room_member.roles.some((r) => r === role.id)) {
									is_mentioned = true;
									break;
								}
							}
						}
					}
				}
			}

			if (
				is_mentioned &&
				notificationPermission() === "granted" &&
				preferences().frontend["desktop_notifs"] === "yes"
			) {
				const author = users.cache.get(m.author_id);
				const channel = channels.cache.get(m.channel_id);
				const title = `${author?.name ?? "Someone"} in #${
					channel?.name ?? "channel"
				}`;
				const rawContent = m.latest_version.type === "DefaultMarkdown"
					? m.latest_version.content ?? ""
					: "";
				const processedContent = await stripMarkdownAndResolveMentions(
					rawContent,
					m.channel_id,
					m.latest_version.mentions,
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

			// TTS notifications
			const ttsEnabled = preferences().frontend["tts_notifs"] === "yes";
			const ttsMode = preferences().notifs.tts;
			const shouldSpeak = ttsEnabled && ttsMode !== "Nothing" &&
				(ttsMode === "Always" || (ttsMode === "Mentions" && is_mentioned));
			const isOwnMessage = m.author_id === users.cache.get("@self")?.id;

			if (
				shouldSpeak && !isOwnMessage &&
				m.latest_version.type === "DefaultMarkdown"
			) {
				const author = users.cache.get(m.author_id);
				const channel = channels.cache.get(m.channel_id);
				const rawContent = m.latest_version.content ?? "";
				const processedContent = await stripMarkdownAndResolveMentions(
					rawContent,
					m.channel_id,
					m.latest_version.mentions,
				);
				const text = processedContent.substring(0, 200);

				const utterance = new SpeechSynthesisUtterance(
					`${author?.name ?? "Someone"} in #${
						channel?.name ?? "channel"
					} says: ${text}`,
				);
				window.speechSynthesis.speak(utterance);
			}

			batch(() => {
				store.messages.handleMessageCreate(m);

				const is_unread = true;
				const t = api.channels.cache.get(m.channel_id);
				if (t) {
					api.channels.cache.set(m.channel_id, {
						...t,
						message_count: (t.message_count ?? 0) + 1,
						mention_count: !is_unread
							? 0
							: (t.mention_count ?? 0) + (is_mentioned ? 1 : 0),
						last_version_id: m.latest_version.version_id,
						last_read_id: !is_unread
							? m.latest_version.version_id
							: t.last_read_id,
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
			});

			for (
				const att of m.latest_version.type === "DefaultMarkdown"
					? m.latest_version.attachments ?? []
					: []
			) {
				if (att.type === "Media") {
					media.cacheInfo.set(att.media.id, att.media);
				}
			}
		} else if (msg.type === "MessageUpdate") {
			const m = msg.message as Message;
			store.messages.handleMessageUpdate(m);
		} else if (msg.type === "MessageDelete") {
			batch(() => {
				const { message_id, channel_id: thread_id } = msg;
				store.messages.handleMessageDelete(thread_id, message_id);

				const ranges = store.messages.cacheRanges.get(thread_id);
				const t = api.channels.cache.get(msg.channel_id);
				if (t) {
					const last_version_id =
						ranges?.live.items.at(-1)?.latest_version.version_id ??
							t.last_version_id;
					console.log({ last_version_id });
					api.channels.cache.set(msg.channel_id, {
						...t,
						message_count: (t.message_count ?? 0) - 1,
						last_version_id,
						is_unread: !!t.last_read_id &&
							t.last_read_id < (last_version_id ?? ""),
					});
				}
			});
		} else if (msg.type === "MessageDeleteBulk") {
			batch(() => {
				const { channel_id: thread_id, message_ids } = msg;
				for (const message_id of message_ids) {
					store.messages.handleMessageDelete(thread_id, message_id);
				}

				const ranges = store.messages.cacheRanges.get(thread_id);
				const t = api.channels.cache.get(thread_id);
				if (t) {
					const last_version_id =
						ranges?.live.items.at(-1)?.latest_version.version_id ??
							t.last_version_id;
					api.channels.cache.set(thread_id, {
						...t,
						message_count: (t.message_count ?? 0) - message_ids.length,
						last_version_id,
						is_unread: !!t.last_read_id &&
							t.last_read_id < (last_version_id ?? ""),
					});
				}
			});
		} else if (msg.type === "MessageVersionDelete") {
			// TODO
		} else if (msg.type === "MediaProcessed") {
			const { media: processedMedia, session_id } = msg;
			media.cacheInfo.set(processedMedia.id, processedMedia);
			// attachment updates are handled by the upload context
		} else if (msg.type === "MediaUpdate") {
			const { media: updatedMedia } = msg;
			media.cacheInfo.set(updatedMedia.id, updatedMedia);
			// attachment updates are handled by the upload context
		} else if (
			msg.type === "RoomMemberCreate" || msg.type === "RoomMemberUpdate"
		) {
			room_members.upsert(msg.member);
			const m = msg.member;
			const l = room_members._cachedListings.get(m.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.user_id === m.user_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: [...p.items.slice(0, idx), m, ...p.items.slice(idx + 1)],
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
						items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
						total: p.total - 1,
					});
				}
			}
		} else if (msg.type === "ThreadMemberUpsert") {
			const { thread_id, added, removed } = msg;

			for (const member of added) {
				thread_members.upsert(member);
				const l = thread_members._cachedListings.get(thread_id);
				if (l?.resource.latest) {
					const p = l.resource.latest;
					const idx = p.items.findIndex((i) => i.user_id === member.user_id);
					if (idx !== -1) {
						l.mutate({
							...p,
							items: [
								...p.items.slice(0, idx),
								member,
								...p.items.slice(idx + 1),
							],
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
							items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
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
						items: [...p.items.slice(0, idx), r, ...p.items.slice(idx + 1)],
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
						items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
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
				const l = invites._cachedChannelListings.get(channel_id);
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
			const ctx_entry = api.ctx?.channel_contexts.get(channel_id);
			if (ctx_entry) {
				const [_ch, chUpdate] = ctx_entry;
				if (slowmode_message_expire_at) {
					const expireDate = new Date(slowmode_message_expire_at);
					chUpdate("slowmode_expire_at", expireDate);
				} else {
					chUpdate("slowmode_expire_at", undefined);
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
								items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
								total: p.total - 1,
							});
						}
					}
				}
			} else if (msg.target.type === "Gdm") {
				const channel_id = msg.target.channel_id;
				const l = invites._cachedChannelListings.get(channel_id);
				if (l?.pagination) {
					const p = l.resource.latest;
					if (p) {
						const idx = p.items.findIndex((i) => i.code === msg.code);
						if (idx !== -1) {
							l.mutate({
								...p,
								items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
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
								items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
								total: p.total - 1,
							});
						}
					}
				}
			}
		} else if (msg.type === "ReactionCreate") {
			const { message_id, channel_id, user_id, key } = msg;
			const message = store.messages.cache.get(message_id);
			if (message) {
				const reactions = [...(message.reactions ?? [])];
				const idx = reactions.findIndex((r) =>
					areReactionKeysEqual(r.key, key)
				);
				if (idx !== -1) {
					const reaction = { ...reactions[idx] };
					reaction.count++;
					if (user_id === getCurrentUserId()) {
						reaction.self = true;
					}
					reactions[idx] = reaction;
				} else {
					reactions.push({
						key,
						count: 1,
						self: user_id === getCurrentUserId(),
					});
				}
				store.messages.cache.set(message_id, { ...message, reactions });
			}
		} else if (msg.type === "ReactionDelete") {
			const { message_id, channel_id, user_id, key } = msg;
			const message = store.messages.cache.get(message_id);
			if (message) {
				const reactions = [...(message.reactions ?? [])];
				const idx = reactions.findIndex((r) =>
					areReactionKeysEqual(r.key, key)
				);
				if (idx !== -1) {
					const reaction = { ...reactions[idx] };
					reaction.count--;
					if (user_id === getCurrentUserId()) {
						reaction.self = false;
					}
					if (reaction.count === 0) {
						reactions.splice(idx, 1);
					} else {
						reactions[idx] = reaction;
					}
				}
				store.messages.cache.set(message_id, { ...message, reactions });
			}
		} else if (msg.type === "ReactionDeleteKey") {
			const { message_id, key } = msg;
			const message = store.messages.cache.get(message_id);
			if (message) {
				const reactions = [...(message.reactions ?? [])];
				const idx = reactions.findIndex((r) =>
					areReactionKeysEqual(r.key, key)
				);
				if (idx !== -1) {
					reactions.splice(idx, 1);
				}
				store.messages.cache.set(message_id, { ...message, reactions });
			}
		} else if (msg.type === "ReactionDeleteAll") {
			const { message_id } = msg;
			const message = store.messages.cache.get(message_id);
			if (message) {
				store.messages.cache.set(message_id, { ...message, reactions: [] });
			}
		} else if (msg.type === "EmojiCreate") {
			// TODO
		} else if (msg.type === "EmojiDelete") {
			// TODO
		} else if (msg.type === "UserCreate" || msg.type === "UserUpdate") {
			users.upsert(msg.user);
		} else if (msg.type === "PresenceUpdate") {
			const { user_id, presence } = msg;
			const user = users.cache.get(user_id);
			if (user) {
				const newUser: UserWithRelationship = {
					...user,
					presence,
				};
				users.cache.set(user_id, newUser);

				if (user_id === users.cache.get("@self")?.id) {
					users.cache.set("@self", newUser);
				}
			}
		} else if (msg.type === "PreferencesGlobal") {
			if (msg.user_id === getCurrentUserId()) {
				if (!deepEqual(preferences(), msg.config)) {
					setPreferences(msg.config);
				}
				setPreferencesLoaded(true);
			}
		} else if (msg.type === "PreferencesRoom") {
			if (msg.user_id === getCurrentUserId()) {
				const room = rooms.cache.get(msg.room_id);
				if (room) {
					rooms.cache.set(msg.room_id, { ...room, preferences: msg.config });
				}
			}
		} else if (msg.type === "PreferencesChannel") {
			if (msg.user_id === getCurrentUserId()) {
				const thread = channels.cache.get(msg.channel_id);
				if (thread) {
					channels.cache.set(thread.id, {
						...thread,
						preferences: msg.config,
					});
				}
			}
		} else if (msg.type === "PreferencesUser") {
			if (msg.user_id === getCurrentUserId()) {
				const user = users.cache.get(msg.target_user_id);
				if (user) {
					const updatedUser: UserWithRelationship = {
						...user,
						preferences: msg.config,
					};
					users.cache.set(msg.target_user_id, updatedUser);
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
				const updatedUser: UserWithRelationship = {
					...user,
					relationship,
				};
				users.cache.set(target_user_id, updatedUser);
			}
		} else if (msg.type === "RelationshipDelete") {
			const { target_user_id } = msg;
			const user = users.cache.get(target_user_id);
			if (user) {
				const updatedUser: UserWithRelationship = {
					...user,
					relationship: {
						relation: null,
						until: null,
						note: null,
						petname: null,
					},
				};
				users.cache.set(target_user_id, updatedUser);
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
			const { ban, room_id } = msg;
			const c = room_bans.cache.get(room_id);
			if (c) {
				c.set(ban.user_id, ban);
			} else {
				room_bans.cache.set(room_id, new ReactiveMap());
				room_bans.cache.get(room_id)!.set(ban.user_id, ban);
			}
			const l = room_bans._cachedListings.get(room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.user_id === ban.user_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: [...p.items.slice(0, idx), ban, ...p.items.slice(idx + 1)],
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
						items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
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
			if (l?.pagination && l.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.id === webhook.id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: [
							...p.items.slice(0, idx),
							webhook,
							...p.items.slice(idx + 1),
						],
					});
				}
			}
		} else if (msg.type === "WebhookDelete") {
			webhooks.cache.delete(msg.webhook_id);
			const l = webhooks._cachedListings.get(msg.channel_id);
			if (l?.pagination && l.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.id === msg.webhook_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: [...p.items.slice(0, idx), ...p.items.slice(idx + 1)],
						total: p.total - 1,
					});
				}
			}
		} else if (msg.type === "InboxNotificationCreate") {
			if (msg.user_id === getCurrentUserId()) {
				inbox.cache.set(msg.notification.id, msg.notification);
				for (const listing of inbox._listings.values()) {
					listing.refetch();
				}
			}
		} else if (msg.type === "InboxMarkRead") {
			if (msg.user_id === getCurrentUserId()) {
				for (const listing of inbox._listings.values()) {
					listing.refetch();
				}
			}
		} else if (msg.type === "InboxMarkUnread") {
			if (msg.user_id === getCurrentUserId()) {
				for (const listing of inbox._listings.values()) {
					listing.refetch();
				}
			}
		} else if (msg.type === "InboxFlush") {
			if (msg.user_id === getCurrentUserId()) {
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
			// Convert User to UserWithRelationship
			const userWithRelationship: UserWithRelationship = {
				...msg.user,
				relationship: {
					note: null,
					relation: null,
					petname: null,
					until: null,
				},
			};
			users.cache.set("@self", userWithRelationship);
			users.cache.set(msg.user.id, userWithRelationship);
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
		mentions?: Message["latest_version"]["mentions"],
	) =>
		stripMarkdownAndResolveMentionsOriginal(content, thread_id, api, mentions);

	const api: Api = {
		rooms,
		channels,
		threads,
		invites,
		webhooks,
		roles,
		room_members,
		room_bans,
		thread_members,
		users,
		media,
		session,
		preferencesLoaded,
		typing,
		tags,
		audit_logs,
		tempCreateSession,
		client,
		clientState,
		emoji,
		reactions,
		dms,
		auth,
		sessions,
		push,
		inbox,
		documents,
		room_analytics,
		voiceStates,
		voiceState,
		stripMarkdownAndResolveMentions,
		ctx: null as any,
		store,
		Provider(props: ParentProps) {
			return (
				<ApiContext.Provider value={api}>
					{props.children}
				</ApiContext.Provider>
			);
		},
		events,
	};

	rooms.api = api;
	channels.api = api;
	threads.api = api;
	roles.api = api;
	room_members.api = api;
	room_bans.api = api;
	thread_members.api = api;
	invites.api = api;
	webhooks.api = api;
	users.api = api;
	audit_logs.api = api;
	media.api = api;
	emoji.api = api;
	reactions.api = api;
	dms.api = api;
	auth.api = api;
	sessions.api = api;
	push.api = api;
	inbox.api = api;
	tags.api = api;
	documents.api = api;
	room_analytics.api = api;

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
	push: Push;
	inbox: Inbox;
	invites: Invites;
	webhooks: Webhooks;
	roles: Roles;
	audit_logs: AuditLogs;
	room_members: RoomMembers;
	room_bans: RoomBans;
	thread_members: ThreadMembers;
	users: Users;
	media: MediaInfo;
	emoji: Emoji;
	reactions: Reactions;
	tags: Tags;
	documents: DocumentsService;
	room_analytics: RoomAnalytics;
	session: Accessor<Session | null>;
	preferencesLoaded: Accessor<boolean>;
	typing: ReactiveMap<string, Set<string>>;
	voiceState: Accessor<VoiceState | null>;
	voiceStates: ReactiveMap<string, VoiceState>;
	tempCreateSession: () => void;
	client: Client;
	clientState: Accessor<ClientState>;
	Provider: Component<ParentProps>;
	ctx: any;
	store: RootStore;

	events: Emitter<{
		sync: [MessageSync, MessageEnvelope];
		ready: MessageReady;
	}>;

	// Utilities
	stripMarkdownAndResolveMentions: (
		content: string,
		thread_id: string,
		mentions?: Message["latest_version"]["mentions"],
	) => Promise<string>;
};

export type Listing<T> = {
	resource: Resource<Pagination<T>>;
	pagination: Pagination<T> | null;
	mutate: (value: Pagination<T>) => void;
	refetch: () => void;
	prom: Promise<unknown> | null;
};
