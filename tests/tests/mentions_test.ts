import { assertEquals } from "@std/assert";
import { BASE_URL, createTester } from "../common.ts";

Deno.test("Mention Parsing", async (t) => {
	const alice = await createTester("alice-mentions");
	const bob = await createTester("bob-mentions");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Mentions Test Room" },
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

	// Create a role
	const role = await alice({
		url: `/room/${roomId}/role`,
		method: "POST",
		body: { name: "test-role" },
		status: 201,
	});
	const roleId = role.id;

	// Invite Bob
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

	// Create an emoji
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
		new Blob([JSON.stringify({ async: false })], { type: "application/json" }),
	);
	const mediaRes = await fetch(`${BASE_URL}/api/v1/media/direct`, {
		method: "POST",
		headers: { "Authorization": `Bearer ${alice.token}` },
		body: formData,
	});
	const { media_id: mediaId } = await mediaRes.json();
	const emoji = await alice({
		url: `/room/${roomId}/emoji`,
		method: "POST",
		body: { name: "mention_me", media_id: mediaId, animated: false },
		status: 200,
	});
	const emojiId = emoji.id;

	await t.step(
		"Alice mentions Bob, a role, a channel, and an emoji",
		async () => {
			const content =
				`Hello <@${bob.user.id}>, check <@&${roleId}> in <#${channelId}> <:mention_me:${emojiId}> @everyone`;
			const message = await alice({
				url: `/channel/${channelId}/message`,
				method: "POST",
				body: { content },
				status: 201,
			});

			const mentions = message.latest_version.mentions;

			// User mentions
			assertEquals(mentions.users.length, 1);
			assertEquals(mentions.users[0].id, bob.user.id);
			assertEquals(mentions.users[0].resolved_name, "bob-mentions");

			// Role mentions
			assertEquals(mentions.roles.length, 1);
			assertEquals(mentions.roles[0].id, roleId);

			// Channel mentions
			assertEquals(mentions.channels.length, 1);
			assertEquals(mentions.channels[0].id, channelId);
			assertEquals(mentions.channels[0].name, "general");

			// Emoji mentions
			assertEquals(mentions.emojis.length, 1);
			assertEquals(mentions.emojis[0].id, emojiId);
			assertEquals(mentions.emojis[0].name, "mention_me");

			// Everyone mention
			assertEquals(mentions.everyone, true);
		},
	);

	await t.step("Alice mentions herself with nickname", async () => {
		// Set nickname for Alice
		await alice({
			url: `/room/${roomId}/member/${alice.user.id}`,
			method: "PATCH",
			body: { override_name: "AliceNick" },
			status: 200,
		});
		const content = `Self mention <@${alice.user.id}>`;
		const message = await alice({
			url: `/channel/${channelId}/message`,
			method: "POST",
			body: { content },
			status: 201,
		});

		const mentions = message.latest_version.mentions;
		assertEquals(mentions.users.length, 1);
		assertEquals(mentions.users[0].id, alice.user.id);
		assertEquals(mentions.users[0].resolved_name, "AliceNick");
	});

	await t.step("Mentions in code blocks are ignored", async () => {
		const content = `\`<@${bob.user.id}>\` and \`@everyone\``;
		const message = await alice({
			url: `/channel/${channelId}/message`,
			method: "POST",
			body: { content },
			status: 201,
		});

		const mentions = message.latest_version.mentions;
		assertEquals(mentions.users.length, 0);
		assertEquals(mentions.everyone, false);
	});
});
