import { admin, createTester } from "../common.ts";
import { assertEquals } from "@std/assert";

const SERVER_ROOM_ID = "00000000-0000-7000-0000-736572766572";

Deno.test("server audit log", async () => {
	const { user: bobUser } = await createTester("bob-server-audit");

	await admin({
		url: `/user/${bobUser.id}/suspend`,
		method: "POST",
		body: {},
		status: 200,
	});

	const logs = await admin({
		url: `/room/${SERVER_ROOM_ID}/audit-logs?dir=b`,
		method: "GET",
		status: 200,
	});

	assertEquals(logs.items.length > 0, true);
	const suspendLog = logs.items.find((item: any) =>
		item.type === "UserSuspend" && item.metadata.user_id === bobUser.id
	);
	assertEquals(suspendLog.metadata.user_id, bobUser.id);
});
