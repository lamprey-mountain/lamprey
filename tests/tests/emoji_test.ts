import { assertEquals } from "@std/assert";
import { BASE_URL, createTester } from "../common.ts";

Deno.test("Custom Emojis", async (t) => {
	const alice = await createTester("alice-emoji");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Emoji Test Room" },
		status: 201,
	});
	const roomId = room.id;

	let mediaId: string;
	let emojiId: string;

	await t.step("Alice uploads an emoji image", async () => {
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
			headers: {
				"Authorization": `Bearer ${alice.token}`,
			},
			body: formData,
		});
		assertEquals(res.status, 201);
		const created = await res.json();
		mediaId = created.media_id;
	});

	await t.step("Alice creates a custom emoji", async () => {
		const emoji = await alice({
			url: `/room/${roomId}/emoji`,
			method: "POST",
			body: {
				name: "test_emoji",
				media_id: mediaId,
				animated: false,
			},
			status: 200, // emoji_create returns 200 Json(emoji) currently
		});
		assertEquals(emoji.name, "test_emoji");
		assertEquals(emoji.media_id, mediaId);
		emojiId = emoji.id;
	});

	await t.step(
		"Alice tries to create an emoji from non-image media",
		async () => {
			const formData = new FormData();
			const data = "not an image";
			formData.append(
				"file",
				new Blob([data], { type: "text/plain" }),
				"test.txt",
			);
			formData.append(
				"json",
				new Blob([JSON.stringify({ async: false })], {
					type: "application/json",
				}),
			);

			const res = await fetch(`${BASE_URL}/api/v1/media/direct`, {
				method: "POST",
				headers: {
					"Authorization": `Bearer ${alice.token}`,
				},
				body: formData,
			});
			assertEquals(res.status, 201);
			const created = await res.json();
			const textMediaId = created.media_id;

			await alice({
				url: `/room/${roomId}/emoji`,
				method: "POST",
				body: {
					name: "fail_emoji",
					media_id: textMediaId,
					animated: false,
				},
				status: 422, // MediaNotAnImage
			});
		},
	);

	await t.step("Alice lists room emojis", async () => {
		const emojis = await alice({
			url: `/room/${roomId}/emoji`,
			status: 200,
		});
		assertEquals(emojis.items.some((e: any) => e.id === emojiId), true);
	});

	await t.step("Alice updates the emoji name", async () => {
		const updated = await alice({
			url: `/room/${roomId}/emoji/${emojiId}`,
			method: "PATCH",
			body: { name: "renamed_emoji" },
			status: 200,
		});
		assertEquals(updated.name, "renamed_emoji");
	});

	await t.step("Alice deletes the emoji", async () => {
		await alice({
			url: `/room/${roomId}/emoji/${emojiId}`,
			method: "DELETE",
			status: 204,
		});

		const emojis = await alice({
			url: `/room/${roomId}/emoji`,
			status: 200,
		});
		assertEquals(emojis.items.some((e: any) => e.id === emojiId), false);
	});
});
