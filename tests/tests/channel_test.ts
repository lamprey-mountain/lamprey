import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Channel Routes and Permissions", async (t) => {
	const alice = await createTester("alice-chan");
	const bob = await createTester("bob-chan");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Channel Test Room", public: false },
		status: 201,
	});
	const roomId = room.id;

	let channelId: string;

	await t.step("Alice creates a text channel", async () => {
		const channel = await alice({
			url: `/room/${roomId}/channel`,
			method: "POST",
			body: { name: "general", type: "Text" },
			status: 201,
		});
		assertEquals(channel.name, "general");
		channelId = channel.id;
	});

	await t.step("Bob cannot see the channel (not in room)", async () => {
		await bob({ url: `/channel/${channelId}`, status: 404 });
	});

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

	await t.step("Bob can now see the channel", async () => {
		const channel = await bob({ url: `/channel/${channelId}`, status: 200 });
		assertEquals(channel.id, channelId);
	});

	await t.step("Bob cannot edit the channel", async () => {
		await bob({
			url: `/channel/${channelId}`,
			method: "PATCH",
			body: { name: "bob-was-here" },
			status: 403,
		});
	});

	await t.step("Alice edits the channel", async () => {
		const updated = await alice({
			url: `/channel/${channelId}`,
			method: "PATCH",
			body: { description: "The main channel" },
			status: 200,
		});
		assertEquals(updated.description, "The main channel");
	});

	await t.step("Alice deletes the channel", async () => {
		await alice({
			url: `/channel/${channelId}/remove`,
			method: "PUT",
			status: 204,
		});
		// TODO: should return 404 for bob, 200 with deleted_at for alice
	});
});
