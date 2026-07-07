import { openDB } from "idb";

async function temp() {
	const db = openDB("cache", 1, {
		// blocked(currentVersion, blockedVersion, event) { },
		// blocking(currentVersion, blockedVersion, event) { },
		// terminated() { },
		upgrade(database, oldVersion, newVersion, transaction, event) {
			// see frontend/src/hooks/useChatClient.ts
			// see frontend/src/lib/sync/db.ts
			// const log = logger.for("idb");
			// for (let i = oldVersion; i < migrations.length; i++) {
			// 	const m = migrations[i];
			// 	m.migrate(db, txn);
			// 	log.info(m.description, undefined, "migrate");
			// }
		},
	});

	// migrations
	const a = await db;
	a.createObjectStore("room", { keyPath: ["id"] }).createIndex(
		"room_idk",
		"foo",
	);
	a.createObjectStore("room_member", { keyPath: ["room_id", "user_id"] });

	const s = a.createObjectStore("channel", { keyPath: ["id"] });
	s.createIndex("channel_by_room", "room_id");
	s.createIndex("channel_by_tag", "tags", { multiEntry: true });

	const sm = a.createObjectStore("message", { keyPath: ["id"] });
	const sv = a.createObjectStore("message_version", {
		keyPath: ["version_id"],
	});
	const sr = a.createObjectStore("message_range", { keyPath: ["id"] });

	sm.createIndex("message_by_channel", "channel_id");
	sv.createIndex("message_version_by_message", "message_id");
	sv.createIndex("message_version_by_channel", "channel_id");
	sr.createIndex("message_range_by_channel", "channel_id");

	// IDBKeyRange.bound(min, max);
	// sm.index("message_by_channel").getAll("channel_id");
}

async function openPublic() {}
async function openPrivate() {}
