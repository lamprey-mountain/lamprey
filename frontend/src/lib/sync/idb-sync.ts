import type { IDBPDatabase, StoreNames } from "idb";
import type {
	Channel,
	MessageReady,
	MessageSync,
	PreferencesGlobal,
	Role,
	RoomMember,
	UserWithRelationship,
} from "sdk";
import type { ApiDB, SyncMeta } from "./db";

export type ApiStore = StoreNames<ApiDB>;

export function hasStore(db: IDBPDatabase<ApiDB>, store: string): boolean {
	return db.objectStoreNames.contains(store as ApiStore);
}

export async function idbGet<T>(
	db: IDBPDatabase<ApiDB>,
	store: string,
	key: IDBValidKey,
): Promise<T | undefined> {
	if (!hasStore(db, store)) return undefined;
	return (await db.get(store as ApiStore, key as never)) as T | undefined;
}

export async function idbPut(
	db: IDBPDatabase<ApiDB>,
	store: string,
	value: unknown,
): Promise<void> {
	if (!hasStore(db, store)) return;
	await db.put(store as ApiStore, value as never);
}

export async function idbDelete(
	db: IDBPDatabase<ApiDB>,
	store: string,
	key: IDBValidKey,
): Promise<void> {
	if (!hasStore(db, store)) return;
	await db.delete(store as ApiStore, key as never);
}

export async function idbClear(
	db: IDBPDatabase<ApiDB>,
	store: string,
): Promise<void> {
	if (!hasStore(db, store)) return;
	await db.clear(store as ApiStore);
}

export async function idbPutMany(
	db: IDBPDatabase<ApiDB>,
	store: string,
	values: unknown[],
): Promise<void> {
	if (!hasStore(db, store) || values.length === 0) return;
	const tx = db.transaction(store as ApiStore, "readwrite");
	for (const value of values) tx.store.put(value as never);
	await tx.done;
}

export async function setSyncMeta(
	db: IDBPDatabase<ApiDB>,
	key: string,
	value: unknown,
): Promise<void> {
	await idbPut(db, "sync_meta", { key, value } satisfies SyncMeta);
}

export async function getSyncMeta<T>(
	db: IDBPDatabase<ApiDB>,
	key: string,
): Promise<T | undefined> {
	if (!hasStore(db, "sync_meta")) return undefined;
	const entry = (await db.get("sync_meta", key)) as SyncMeta | undefined;
	return entry?.value as T | undefined;
}

export async function applyReadyToDb(
	db: IDBPDatabase<ApiDB>,
	ready: MessageReady,
): Promise<void> {
	await setSyncMeta(db, "ready", ready);
	if (ready.session) await idbPut(db, "session", ready.session);
}

export async function applySyncToDb(
	db: IDBPDatabase<ApiDB>,
	event: MessageSync,
	currentUserId?: string,
): Promise<void> {
	switch (event.type) {
		case "Ambient": {
			await idbPutMany(db, "room", event.rooms);
			await idbPutMany(db, "channel", [...event.channels, ...event.threads]);
			await idbPutMany(db, "role", event.roles);
			await idbPutMany(db, "room_member", event.room_members);
			if (event.config) await setSyncMeta(db, "config", event.config);
			break;
		}
		case "RoomCreate":
		case "RoomUpdate":
			await idbPut(db, "room", event.room);
			break;
		case "RoomDelete": {
			// PERF: batch deletes
			await idbDelete(db, "room", event.room_id);
			const [channels, roles, members] = await Promise.all([
				db.getAll("channel"),
				db.getAll("role"),
				db.getAll("room_member"),
			]);
			for (const channel of channels) {
				if (channel.room_id === event.room_id) {
					await idbDelete(db, "channel", channel.id);
				}
			}
			for (const role of roles) {
				if (role.room_id === event.room_id) {
					await idbDelete(db, "role", role.id);
				}
			}
			for (const member of members) {
				if (member.room_id === event.room_id) {
					await idbDelete(db, "room_member", [member.room_id, member.user_id]);
				}
			}
			break;
		}
		case "ChannelCreate":
		case "ChannelUpdate":
			await idbPut(db, "channel", event.channel);
			break;
		case "ChannelReorder": {
			for (const item of event.channels) {
				const channel = await db.get("channel", item.id);
				if (!channel) continue;
				await idbPut(db, "channel", {
					...channel,
					parent_id: item.parent_id,
					position: item.position,
				});
			}
			break;
		}
		case "RoleCreate":
		case "RoleUpdate":
			await idbPut(db, "role", event.role);
			break;
		case "RoleDelete":
			await idbDelete(db, "role", event.role_id);
			break;
		case "RoleReorder": {
			for (const item of event.roles) {
				const role = await db.get("role", item.role_id);
				if (!role) continue;
				await idbPut(db, "role", {
					...role,
					position: item.position,
				});
			}
			break;
		}
		case "RoomMemberCreate":
		case "RoomMemberUpdate":
			await idbPut(db, "room_member", event.member);
			await idbPut(db, "user", event.user);
			break;
		case "RoomMemberDelete":
			await idbDelete(db, "room_member", [event.room_id, event.user_id]);
			break;
		case "ThreadMemberUpsert":
			await idbPutMany(db, "thread_member", event.added);
			for (const user_id of event.removed) {
				await idbDelete(db, "thread_member", [event.thread_id, user_id]);
			}
			break;
		case "MessageCreate": {
			await idbPut(db, "message", event.message);
			const thread = (event.message as { thread?: Channel }).thread;
			if (thread) await idbPut(db, "channel", thread);
			break;
		}
		case "MessageUpdate":
			await idbPut(db, "message", event.message);
			break;
		case "MessageDelete":
			await idbDelete(db, "message", event.message_id);
			break;
		case "MessageDeleteBulk":
			for (const message_id of event.message_ids) {
				await idbDelete(db, "message", message_id);
			}
			break;
		case "UserCreate":
		case "UserUpdate": {
			const existing = await db.get("user", event.user.id);
			const relationship = (existing as UserWithRelationship | undefined)
				?.relationship;
			await idbPut(db, "user", {
				...(existing ?? {}),
				...event.user,
				...(relationship ? { relationship } : {}),
			});
			break;
		}
		case "PresenceUpdate": {
			const user = await db.get("user", event.user_id);
			if (user) await idbPut(db, "user", { ...user, presence: event.presence });
			break;
		}
		case "PreferencesGlobal":
			if (event.user_id === currentUserId) {
				await setSyncMeta(db, "config", event.config);
			}
			break;
		case "SessionCreate":
		case "SessionUpdate":
			await idbPut(db, "session", event.session);
			break;
		case "SessionDelete":
			await idbDelete(db, "session", event.id);
			break;
		case "SessionDeleteAll":
			await db.clear("session");
			break;
		case "InviteCreate":
		case "InviteUpdate":
			await idbPut(db, "invite", event.invite);
			break;
		case "InviteDelete":
			await idbDelete(db, "invite", event.code);
			break;
		case "EmojiCreate":
		case "EmojiUpdate":
			await idbPut(db, "emoji", event.emoji);
			break;
		case "EmojiDelete":
			await idbDelete(db, "emoji", event.emoji_id);
			break;
		case "TagCreate":
		case "TagUpdate":
			await idbPut(db, "tag", event.tag);
			break;
		case "TagDelete":
			await idbDelete(db, "tag", event.tag_id);
			break;
		case "WebhookCreate":
		case "WebhookUpdate":
			await idbPut(db, "webhook", event.webhook);
			break;
		case "WebhookDelete":
			await idbDelete(db, "webhook", event.webhook_id);
			break;
		case "BanCreate":
			await idbPut(db, "room_ban", event.ban);
			break;
		case "BanDelete":
			await idbDelete(db, "room_ban", [event.room_id, event.user_id]);
			break;
		case "AuditLogEntryCreate":
			await idbPut(db, "audit_log", event.entry);
			break;
		case "MediaProcessed":
		case "MediaUpdate":
			await idbPut(db, "media", event.media);
			break;
		case "ScriptCreate":
		case "ScriptUpdate":
			await idbPut(db, "script", event.script);
			break;
		case "ScriptDelete":
			await idbDelete(
				db,
				"script",
				"redex_id" in event ? event.redex_id : event.script_id,
			);
			break;
		case "ScriptVersionCreate":
		case "ScriptVersionUpdate": {
			const script = await db.get("script", event.redex_id);
			if (script) {
				await idbPut(db, "script", {
					...script,
					latest_version: event.version,
				});
			}
			break;
		}
		case "ScriptRunCreate":
		case "ScriptRunUpdate":
			await idbPut(db, "script_run", event.run);
			break;
	}
}

function isThread(channel: Channel): boolean {
	return (
		channel.type === "ThreadPublic" ||
		channel.type === "ThreadPrivate" ||
		channel.type === "ThreadForum2"
	);
}

export async function readAmbientFromDb(
	db: IDBPDatabase<ApiDB>,
	user_id: string,
	config: PreferencesGlobal,
): Promise<MessageSync | null> {
	const [rooms, channels, roles, members] = await Promise.all([
		db.getAll("room"),
		db.getAll("channel"),
		db.getAll("role"),
		db.getAll("room_member"), // PERF: i only need my own room member
	]);

	const threads = channels.filter(
		(channel) => isThread(channel) && !channel.archived_at,
	);
	const room_members = members.filter((member) => member.user_id === user_id);

	return {
		type: "Ambient",
		user_id,
		rooms,
		roles: roles as Role[],
		channels: channels.filter((channel) => !isThread(channel)),
		threads,
		room_members: room_members as RoomMember[],
		config,
	};
}
