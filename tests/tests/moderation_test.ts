import { assertEquals } from "@std/assert";
import { BASE_URL, createTester } from "../common.ts";

Deno.test("Moderation (Kick, Ban, Timeout)", async (t) => {
	const alice = await createTester("alice-mod");
	const bob = await createTester("bob-mod");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Moderation Test Room", public: false },
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

	await t.step("Alice kicks Bob", async () => {
		// Test permission: Bob tries to kick Alice and fails
		await bob({
			url: `/room/${roomId}/member/${alice.user.id}`,
			method: "DELETE",
			status: 403,
		});

		await alice({
			url: `/room/${roomId}/member/${bob.user.id}`,
			method: "DELETE",
			status: 204,
		});

		// Bob should no longer see the room
		await bob({ url: `/room/${roomId}`, status: 404 });
	});

	await t.step("Bob rejoins via invite", async () => {
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

	await t.step("Alice bans Bob", async () => {
		// Test permission: Bob tries to ban Alice and fails
		await bob({
			url: `/room/${roomId}/ban/${alice.user.id}`,
			method: "PUT",
			body: { reason: "i am bob" },
			status: 403,
		});

		await alice({
			url: `/room/${roomId}/ban/${bob.user.id}`,
			method: "PUT",
			body: { reason: "test ban" },
			status: 204,
		});

		// Bob should no longer see the room
		await bob({ url: `/room/${roomId}`, status: 404 });

		// Bob tries to rejoin and fails
		const invite = await alice({
			url: `/room/${roomId}/invite`,
			method: "POST",
			body: {},
			status: 201,
		});
		await bob({
			url: `/invite/${invite.code}`,
			method: "POST",
			status: 403, // YouAreBanned
		});
	});

	await t.step("Alice unbans Bob", async () => {
		await alice({
			url: `/room/${roomId}/ban/${bob.user.id}`,
			method: "DELETE",
			status: 204,
		});

		// Bob can now join
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

	await t.step("Alice times out Bob", async () => {
		const timeoutUntil = new Date(Date.now() + 5000).toISOString();
		await alice({
			url: `/room/${roomId}/member/${bob.user.id}`,
			method: "PATCH",
			body: { timeout_until: timeoutUntil },
			status: 200,
		});

		const channel = await alice({
			url: `/room/${roomId}/channel`,
			method: "POST",
			body: { name: "timeout-chat", type: "Text" },
			status: 201,
		});

		// Bob tries to send a message and fails
		await bob({
			url: `/channel/${channel.id}/message`,
			method: "POST",
			body: { content: "i am timed out" },
			status: 403, // Forbidden (timed out)
		});

		// Wait for timeout to expire
		await new Promise((r) => setTimeout(r, 6000));

		// Bob can now send a message
		await bob({
			url: `/channel/${channel.id}/message`,
			method: "POST",
			body: { content: "i am free" },
			status: 201,
		});
	});
});
