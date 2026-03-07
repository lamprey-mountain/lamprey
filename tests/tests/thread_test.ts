import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Threads", async (t) => {
	const alice = await createTester("alice-th");
	const bob = await createTester("bob-th");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Thread Test Room", public: true },
		status: 201,
	});

	let channelId: string;
	await t.step("Alice creates a text channel", async () => {
		const channel = await alice({
			url: `/room/${room.id}/channel`,
			method: "POST",
			body: { name: "general", type: "Text" },
			status: 201,
		});
		channelId = channel.id;
	});

	await t.step("Bob joins the room", async () => {
		const invite = await alice({
			url: `/room/${room.id}/invite`,
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

	let threadId: string;

	await t.step("Alice creates a thread from a message", async () => {
		const message = await alice({
			url: `/channel/${channelId}/message`,
			method: "POST",
			body: { content: "let's discuss this" },
			status: 201,
		});

		const thread = await alice({
			url: `/channel/${channelId}/message/${message.id}/thread`,
			method: "POST",
			body: { name: "Discussion Thread", ty: "ThreadPublic" },
			status: 201,
		});
		assertEquals(thread.name, "Discussion Thread");
		threadId = thread.id;

		// Join the thread (might be 304 if auto-joined)
		await alice({
			url: `/thread/${threadId}/member/@self`,
			method: "PUT",
			body: {},
			status: 200,
		});
	});

	await t.step("Bob joins the thread", async () => {
		await bob({
			url: `/thread/${threadId}/member/@self`,
			method: "PUT",
			body: {},
			status: 200,
		});

		const members = await alice({
			url: `/thread/${threadId}/member`,
			status: 200,
		});
		assertEquals(
			members.items.some((m: any) => m.user_id === bob.user.id),
			true,
		);
	});

	await t.step("Bob sends a message in the thread", async () => {
		const message = await bob({
			url: `/channel/${threadId}/message`,
			method: "POST",
			body: { content: "i agree" },
			status: 201,
		});
		assertEquals(message.latest_version.content, "i agree");
	});

	await t.step("Alice leaves the thread", async () => {
		await alice({
			url: `/thread/${threadId}/member/@self`,
			method: "DELETE",
			status: 204,
		});

		const members = await bob({
			url: `/thread/${threadId}/member`,
			status: 200,
		});
		assertEquals(
			members.items.some((m: any) => m.user_id === alice.user.id),
			false,
		);
	});
});
