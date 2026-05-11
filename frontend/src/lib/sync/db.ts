import {
	type DBSchema,
	deleteDB,
	type IDBPDatabase,
	type IDBPTransaction,
	type StoreNames,
} from "idb";
import type {
	AuditLogEntry,
	EmojiCustom,
	Invite,
	Notification,
	PushInfo,
	RoomBan,
	Run,
	RunLogEntry,
	Script,
	Tag,
	ThreadMember,
	Webhook,
} from "sdk";
import type { RevisionContent } from "@/api/services/DocumentsService.ts";
import type {
	ChannelT,
	MediaT,
	MemberT,
	MessageT,
	RoleT,
	RoomT,
	SessionT,
	UserT,
} from "@/types";

export interface ApiDB extends DBSchema {
	user: {
		value: UserT;
		key: string;
	};
	room: {
		value: RoomT;
		key: string;
	};
	channel: {
		value: ChannelT;
		key: string;
	};
	message: {
		value: MessageT;
		key: string;
		indexes: { channel_id: string };
	};
	role: {
		value: RoleT;
		key: string;
	};
	room_member: {
		value: MemberT;
		key: [string, string];
	};
	media: {
		value: MediaT;
		key: string;
	};
	session: {
		value: SessionT;
		key: string;
	};
	document: {
		value: RevisionContent;
		key: string;
	};
	thread_member: {
		value: ThreadMember;
		key: [string, string];
	};
	message_ranges: {
		value: IDBMessageRange;
		key: string;
		indexes: { channel_id: string };
	};
	dm: {
		value: ChannelT;
		key: string;
	};
	invite: {
		value: Invite;
		key: string;
	};
	webhook: {
		value: Webhook;
		key: string;
	};
	emoji: {
		value: EmojiCustom;
		key: string;
	};
	notification: {
		value: Notification;
		key: string;
	};
	push: {
		value: PushInfo;
		key: string;
	};
	room_ban: {
		value: RoomBan;
		key: [string, string];
	};
	tag: {
		value: Tag;
		key: string;
	};
	audit_log: {
		value: AuditLogEntry;
		key: string;
	};
	script: {
		value: Script;
		key: string;
		indexes: { channel_id: string };
	};
	script_run: {
		value: Run;
		key: string;
		indexes: { script_id: string };
	};
	script_log: {
		value: RunLogEntry;
		key: [string, number];
		indexes: { run_id: string };
	};
}

interface IDBMessageRange {
	/** random uuid for this range */
	id: string;
	channel_id: string;

	/** id of the first message in this range */
	start_id: string;

	/** id of the last message in this range */
	end_id: string;

	has_backwards: boolean;
	has_forward: boolean;
}

export type Migration = {
	description: string;
	migrate(
		db: IDBPDatabase<ApiDB>,
		tx: IDBPTransaction<ApiDB, StoreNames<ApiDB>[], "versionchange">,
	): void;
};

export const migrations: Array<Migration> = [
	{
		description: "stores for various resources",
		migrate(db) {
			db.createObjectStore("user", { keyPath: "id" });
			db.createObjectStore("room", { keyPath: "id" });
			db.createObjectStore("channel", { keyPath: "id" });
			db.createObjectStore("message", { keyPath: "id" });
			db.createObjectStore("role", { keyPath: "id" });
			db.createObjectStore("room_member", {
				keyPath: ["room_id", "user_id"],
			});
			db.createObjectStore("media", { keyPath: "id" });
			db.createObjectStore("session", { keyPath: "id" });
			db.createObjectStore("document", { keyPath: "id" });
			db.createObjectStore("thread_member", {
				keyPath: ["thread_id", "user_id"],
			});
		},
	},
	{
		description: "channel id index for messages",
		migrate(_db, txn) {
			txn.objectStore("message").createIndex("channel_id", "channel_id");
		},
	},
	{
		description: "stores for message ranges",
		migrate(db) {
			const ranges = db.createObjectStore("message_ranges", { keyPath: "id" });
			ranges.createIndex("channel_id", "channel_id");
		},
	},
	{
		description:
			"add stores for DMs, invites, webhooks, emoji, and other resources",
		migrate(db) {
			db.createObjectStore("dm", { keyPath: "id" });
			db.createObjectStore("invite", { keyPath: "code" });
			db.createObjectStore("webhook", { keyPath: "id" });
			db.createObjectStore("emoji", { keyPath: "id" });
			db.createObjectStore("notification", { keyPath: "id" });
			db.createObjectStore("push", { keyPath: "id" });
			db.createObjectStore("room_ban", {
				keyPath: ["room_id", "user_id"],
			});
			db.createObjectStore("tag", { keyPath: "id" });
			db.createObjectStore("audit_log", { keyPath: "id" });
		},
	},
	{
		description: "stores for scripts, runs, and logs",
		migrate(db) {
			const script = db.createObjectStore("script", { keyPath: "id" });
			script.createIndex("channel_id", "channel_id");

			const run = db.createObjectStore("script_run", { keyPath: "id" });
			run.createIndex("script_id", "script_id");

			const log = db.createObjectStore("script_log", {
				keyPath: ["run_id", "seq"],
			});
			log.createIndex("run_id", "run_id");
		},
	},
];

/** Clears all data from all object stores in the database */
export async function clearApiDatabase(db: IDBPDatabase<ApiDB>) {
	const storeNames = Array.from(db.objectStoreNames) as StoreNames<ApiDB>[];
	const tx = db.transaction(storeNames, "readwrite");

	await Promise.all(
		storeNames.map((name) => {
			const store = tx.objectStore(name);
			return store.clear();
		}),
	);

	return tx.done;
}
