import { createTester } from "../common.ts";
import { assertEquals } from "@std/assert";

Deno.test("user audit log", async () => {
	const { tester: alice } = await createTester("alice-user-audit");

	await alice({
		url: `/user/@self`,
		method: "PATCH",
		body: { name: "alice-new-name" },
		status: 200,
	});

	const logs = await alice({
		url: `/user/@self/audit-logs`,
		method: "GET",
		status: 200,
	});

	assertEquals(logs.items.length > 0, true);
	const userUpdateLog = logs.items.find((item: any) =>
		item.type === "UserUpdate"
	);
	assertEquals(userUpdateLog.metadata.changes[0].key, "name");
	assertEquals(userUpdateLog.metadata.changes[0].old, "alice-user-audit");
	assertEquals(userUpdateLog.metadata.changes[0].new, "alice-new-name");
});
