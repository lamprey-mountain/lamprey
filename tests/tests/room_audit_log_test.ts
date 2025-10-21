import { createTester } from "../common.ts";
import { assertEquals } from "@std/assert";

Deno.test("room audit log", async () => {
	const { tester: alice } = await createTester("alice-room-audit");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "room-audit-test" },
		status: 201,
	});

	await alice({
		url: `/room/${room.id}`,
		method: "PATCH",
		body: { name: "room-audit-test-renamed" },
		status: 200,
	});

	const logs = await alice({
		url: `/room/${room.id}/audit-logs`,
		method: "GET",
		status: 200,
	});

	assertEquals(logs.items.length > 0, true);
	const roomUpdateLog = logs.items.find((item: any) =>
		item.type === "RoomUpdate"
	);
	assertEquals(roomUpdateLog.metadata.changes[0].key, "name");
	assertEquals(roomUpdateLog.metadata.changes[0].old, "room-audit-test");
	assertEquals(
		roomUpdateLog.metadata.changes[0].new,
		"room-audit-test-renamed",
	);
});
