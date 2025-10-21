import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("user routes", async (t) => {
	const { tester: regular, token: regularToken } = await createTester(
		"user-tester",
	);

	await t.step("get root user", async () => {
		const user0 = await regular({
			url: `/user/00000000-0000-7000-0000-0000726f6f74`,
			method: "GET",
			status: 200,
		});

		// fetching a user returns their id
		assertEquals(user0.id, "00000000-0000-7000-0000-0000726f6f74");

		// root user is marked as system
		assertEquals(user0.system, true);

		// root user is not deletable
		assertEquals(user0.deleted_at, null);
	});

	await t.step("get self", async () => {
		// we can fetch ourselves
		const _user1 = await regular({
			url: `/user/@self`,
			method: "GET",
			status: 200,
		});
	});
});
