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

Deno.test("room audit log with filter", async () => {
	const { tester: alice, testerWithUser: createBob } = await createTester(
		"alice-room-audit-filter",
	);
	const { tester: bob, user: bobUser } = await createBob("bob-room-audit-filter");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "room-audit-filter-test" },
		status: 201,
	});

	// invite bob
	const invite = await alice({
		url: `/room/${room.id}/invite`,
		method: "POST",
		body: {},
		status: 200,
	});

	await bob({
		url: `/invite/${invite.code}`,
		method: "POST",
		body: {},
		status: 200,
	});

	// alice updates room name
	await alice({
		url: `/room/${room.id}`,
		method: "PATCH",
		body: { name: "room-audit-filter-test-renamed" },
		status: 200,
	});

	// alice creates a role for bob
	const role = await alice({
		url: `/room/${room.id}/role`,
		method: "POST",
		body: {
			name: "test-role",
			permissions: ["RoomManage"],
		},
		status: 201,
	});

	// alice assigns role to bob
	await alice({
		url: `/room/${room.id}/member/${bobUser.id}/role/${role.id}`,
		method: "PUT",
		status: 204,
	});

	// bob updates room description
	await bob({
		url: `/room/${room.id}`,
		method: "PATCH",
		body: { description: "bob was here" },
		status: 200,
	});

	const aliceUser = await alice({ url: "/users/@me", method: "GET", status: 200 });

	// fetch logs with alice's user_id
	const aliceLogs = await alice({
		url: `/room/${room.id}/audit-logs?user_id=${aliceUser.id}`,
		method: "GET",
		status: 200,
	});
	assertEquals(aliceLogs.items.length > 0, true);
	assertEquals(
		aliceLogs.items.every((item: any) => item.user_id === aliceUser.id),
		true,
	);

	// fetch logs with bob's user_id
	const bobLogs = await alice({
		url: `/room/${room.id}/audit-logs?user_id=${bobUser.id}`,
		method: "GET",
		status: 200,
	});
	assertEquals(bobLogs.items.length, 1);
	assertEquals(bobLogs.items[0].type, "RoomUpdate");
	assertEquals(bobLogs.items[0].user_id, bobUser.id);

	// fetch logs with type=RoomUpdate
	const roomUpdateLogs = await alice({
		url: `/room/${room.id}/audit-logs?type=RoomUpdate`,
		method: "GET",
		status: 200,
	});
	assertEquals(roomUpdateLogs.items.length, 2);
	assertEquals(
		roomUpdateLogs.items.some((item: any) => item.user_id === aliceUser.id),
		true,
	);
	assertEquals(
		roomUpdateLogs.items.some((item: any) => item.user_id === bobUser.id),
		true,
	);
});