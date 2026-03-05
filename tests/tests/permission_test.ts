import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Permissions and Access Control", async (t) => {
	const alice = await createTester("alice-perms");
	const bob = await createTester("bob-perms");

	// Alice creates a room
	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Perms Test Room", public: false },
		status: 201,
	});
	const roomId = room.id;

	await t.step(
		"Bob cannot see Alice's private room without invite",
		async () => {
			await bob({ url: `/room/${roomId}`, status: 404 });
		},
	);

	await t.step("Alice invites Bob", async () => {
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

	await t.step("Bob can now see the room", async () => {
		const roomInfo = await bob({ url: `/room/${roomId}`, status: 200 });
		assertEquals(roomInfo.id, roomId);
	});

	await t.step("Bob cannot delete Alice's room", async () => {
		await bob({
			url: `/room/${roomId}`,
			method: "DELETE",
			status: 403,
		});
	});

	await t.step("Bob cannot edit Alice's room", async () => {
		await bob({
			url: `/room/${roomId}`,
			method: "PATCH",
			body: { name: "Bob's Room" },
			status: 403,
		});
	});
});
