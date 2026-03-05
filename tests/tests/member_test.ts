import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Room Member Operations", async (t) => {
	const alice = await createTester("alice-mem");
	const bob = await createTester("bob-mem");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Member Test Room", public: false },
		status: 201,
	});
	const roomId = room.id;

	await t.step("Bob joins via invite", async () => {
		const invite = await alice({
			url: `/room/${roomId}/invite`,
			method: "POST",
			body: {},
			status: 201,
		});
		await bob({
			url: `/invite/${invite.code}`,
			method: "POST",
			status: 204,
		});
	});

	await t.step("Alice lists room members", async () => {
		const members = await alice({
			url: `/room/${roomId}/member`,
			status: 200,
		});
		assertEquals(members.items.length, 2);
	});

	await t.step("Bob updates his room nickname", async () => {
		const updated = await bob({
			url: `/room/${roomId}/member/@self`,
			method: "PATCH",
			body: { override_name: "Bob the Builder" },
			status: 200,
		});
		assertEquals(updated.override_name, "Bob the Builder");
	});

	await t.step("Alice kicks Bob", async () => {
		await alice({
			url: `/room/${roomId}/member/${bob.user.id}`,
			method: "DELETE",
			status: 204,
		});

		// Bob should no longer see the room
		await bob({ url: `/room/${roomId}`, status: 404 });
	});
});
