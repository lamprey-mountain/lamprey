import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Message Routes and Permissions", async (t) => {
	const alice = await createTester("alice-msg");
	const bob = await createTester("bob-msg");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Message Test Room", public: false },
		status: 201,
	});
	const roomId = room.id;

	const channel = await alice({
		url: `/room/${roomId}/channel`,
		method: "POST",
		body: { name: "general", type: "Text" },
		status: 201,
	});
	const channelId = channel.id;

	let messageId: string;

	await t.step("Alice sends a message", async () => {
		const message = await alice({
			url: `/channel/${channelId}/message`,
			method: "POST",
			body: { content: "hello world" },
			status: 201,
		});
		assertEquals(message.latest_version.content, "hello world");
		messageId = message.id;
	});

	await t.step("Bob cannot send message (not in room)", async () => {
		await bob({
			url: `/channel/${channelId}/message`,
			method: "POST",
			body: { content: "bob message" },
			status: 404,
		});
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

	await t.step("Bob can now send a message", async () => {
		const message = await bob({
			url: `/channel/${channelId}/message`,
			method: "POST",
			body: { content: "hi alice" },
			status: 201,
		});
		assertEquals(message.latest_version.content, "hi alice");
	});

	await t.step("Alice updates her message", async () => {
		const updated = await alice({
			url: `/channel/${channelId}/message/${messageId}`,
			method: "PATCH",
			body: { content: "hello updated" },
			status: 200,
		});
		assertEquals(updated.latest_version.content, "hello updated");
	});

	await t.step("Bob cannot update Alice's message", async () => {
		await bob({
			url: `/channel/${channelId}/message/${messageId}`,
			method: "PATCH",
			body: { content: "hacked by bob" },
			status: 403,
		});
	});

	await t.step("Alice deletes her message", async () => {
		await alice({
			url: `/channel/${channelId}/message/${messageId}`,
			method: "DELETE",
			status: 204,
		});
	});
});
