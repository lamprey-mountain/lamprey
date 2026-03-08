import { assertEquals } from "@std/assert";
import { BASE_URL, createTester } from "../common.ts";

Deno.test("Emoji Use External Permission", async (t) => {
	const alice = await createTester("alice-external-emoji");

	// Room A: Where the emoji lives
	const roomA = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Room A" },
		status: 201,
	});
	const roomAId = roomA.id;

	// Room B: Where we want to use it
	const roomB = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Room B" },
		status: 201,
	});
	const roomBId = roomB.id;

	// Channel in Room B
	const channelB = await alice({
		url: `/room/${roomBId}/channel`,
		method: "POST",
		body: { name: "general", type: "Text" },
		status: 201,
	});
	const channelBId = channelB.id;

	// Create emoji in Room A
	const pngData = Uint8Array.from(
		atob(
			"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==",
		),
		(c) => c.charCodeAt(0),
	);
	const formData = new FormData();
	formData.append(
		"file",
		new Blob([pngData], { type: "image/png" }),
		"emoji.png",
	);
	formData.append(
		"json",
		new Blob([JSON.stringify({ async: false })], {
			type: "application/json",
		}),
	);
	const res = await fetch(`${BASE_URL}/api/v1/media/direct`, {
		method: "POST",
		headers: { "Authorization": `Bearer ${alice.token}` },
		body: formData,
	});
	assertEquals(res.status, 201);
	const { media_id: mediaId } = await res.json();
	const emoji = await alice({
		url: `/room/${roomAId}/emoji`,
		method: "POST",
		body: { name: "external_test", media_id: mediaId, animated: false },
		status: 200,
	});
	const emojiId = emoji.id;
	const emojiStr = `<:external_test:${emojiId}>`;

	// 1. Explicitly deny EmojiUseExternal to @everyone in Room B
	await alice({
		url: `/room/${roomBId}/role/${roomBId}`, // @everyone role id is room id
		method: "PATCH",
		body: { deny: ["EmojiUseExternal"] },
		status: 200,
	});

	await t.step(
		"Alice cannot use Room A emoji in Room B when permission is denied",
		async () => {
			const message = await alice({
				url: `/channel/${channelBId}/message`,
				method: "POST",
				body: { content: `hello ${emojiStr}` },
				status: 201,
			});
			// Emoji should be stripped to :external_test:
			assertEquals(message.latest_version.content, "hello :external_test:");
		},
	);

	// 2. Grant EmojiUseExternal back to @everyone in Room B (remove from deny)
	await alice({
		url: `/room/${roomBId}/role/${roomBId}`,
		method: "PATCH",
		body: { allow: ["EmojiUseExternal"], deny: [] },
		status: 200,
	});

	await t.step("Alice can now use Room A emoji in Room B", async () => {
		const message = await alice({
			url: `/channel/${channelBId}/message`,
			method: "POST",
			body: { content: `hello ${emojiStr}` },
			status: 201,
		});
		// Emoji should be kept
		assertEquals(message.latest_version.content, `hello ${emojiStr}`);
	});

	await t.step("Alice can use Room A emoji in edit in Room B", async () => {
		const message = await alice({
			url: `/channel/${channelBId}/message`,
			method: "POST",
			body: { content: "plain text" },
			status: 201,
		});
		const messageId = message.id;

		const updated = await alice({
			url: `/channel/${channelBId}/message/${messageId}`,
			method: "PATCH",
			body: { content: `edited ${emojiStr}` },
			status: 200,
		});
		// Emoji should be kept
		assertEquals(updated.latest_version.content, `edited ${emojiStr}`);
	});
});
