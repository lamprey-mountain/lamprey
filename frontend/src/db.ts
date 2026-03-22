import { DBSchema, type IDBPDatabase, openDB } from "idb";
import {
	ChannelT,
	MediaT,
	MemberT,
	MessageT,
	RoleT,
	RoomT,
	SessionT,
	UserT,
} from "./types.ts";
import type { RevisionContent } from "./api/services/DocumentsService.ts";
import type { ThreadMember } from "sdk";

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
}

export type Migration = {
	description: string;
	migrate(db: IDBPDatabase<ApiDB>): void;
};

export const migrations: Array<Migration> = [
	{
		description: "stores for various resources",
		migrate(database) {
			database.createObjectStore("user", { keyPath: "id" });
			database.createObjectStore("room", { keyPath: "id" });
			database.createObjectStore("channel", { keyPath: "id" });
			database.createObjectStore("message", { keyPath: "id" });
			database.createObjectStore("role", { keyPath: "id" });
			database.createObjectStore("room_member", {
				keyPath: ["room_id", "user_id"],
			});
			database.createObjectStore("media", { keyPath: "id" });
			database.createObjectStore("session", { keyPath: "id" });
			database.createObjectStore("document", { keyPath: "id" });
			database.createObjectStore("thread_member", {
				keyPath: ["thread_id", "user_id"],
			});
		},
	},
];
