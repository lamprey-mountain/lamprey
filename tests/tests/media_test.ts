import { assertEquals } from "@std/assert";
import { BASE_URL, createTester, SyncClient } from "../common.ts";

Deno.test("Media Uploads", async (t) => {
	const alice = await createTester("alice-me");

	let mediaId: string;
	let uploadUrl: string;

	await t.step("Alice creates a media upload", async () => {
		const res = await fetch(`${BASE_URL}/api/v1/media`, {
			method: "POST",
			headers: {
				"Authorization": `Bearer ${alice.token}`,
				"Content-Type": "application/json",
			},
			body: JSON.stringify({
				filename: "test.txt",
				size: 11,
				strip_exif: false,
			}),
		});
		assertEquals(res.status, 201);
		const data = await res.json();
		mediaId = data.media_id;
		uploadUrl = data.upload_url;
		assertEquals(res.headers.get("upload-offset"), "0");
		assertEquals(res.headers.get("upload-length"), "11");
	});

	await t.step("Alice uploads data to the media", async () => {
		const res = await fetch(uploadUrl, {
			method: "PATCH",
			headers: {
				"Authorization": `Bearer ${alice.token}`,
				"upload-offset": "0",
				"content-length": "11",
			},
			body: "hello world",
		});
		assertEquals(res.status, 204);
		assertEquals(res.headers.get("upload-offset"), "11");
	});

	await t.step(
		"Alice marks media as done (optional if fully uploaded)",
		async () => {
			// If PATCH was full size, it already started processing.
			// Let's just wait for it to be ready by polling media_get.
			let media;
			for (let i = 0; i < 20; i++) {
				const res = await fetch(`${BASE_URL}/api/v1/media/${mediaId}`, {
					headers: { "Authorization": `Bearer ${alice.token}` },
				});
				if (res.status === 200) {
					const m = await res.json();
					if (m.status === "Uploaded" || m.status === "Consumed") {
						media = m;
						break;
					}
				}
				await new Promise((r) => setTimeout(r, 500));
			}
			assertEquals(media?.id, mediaId);
			assertEquals(media?.size, 11);
		},
	);

	await t.step("Alice deletes the media", async () => {
		await alice({
			url: `/media/${mediaId}`,
			method: "DELETE",
			status: 204,
		});

		// Verify deletion
		const res = await fetch(`${BASE_URL}/api/v1/media/${mediaId}`, {
			headers: { "Authorization": `Bearer ${alice.token}` },
		});
		assertEquals(res.status, 404);
	});

	await t.step("Alice uploads via direct upload (sync)", async () => {
		const formData = new FormData();
		const data = "direct sync upload data";
		formData.append(
			"file",
			new Blob([data], { type: "text/plain" }),
			"direct-sync.txt",
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
		const media = await res.json();
		// media_upload_direct returns MediaCreated which has media_id
		assertEquals(typeof media.media_id, "string");

		// verify its actually processed by getting it
		const getRes = await fetch(`${BASE_URL}/api/v1/media/${media.media_id}`, {
			headers: { "Authorization": `Bearer ${alice.token}` },
		});
		assertEquals(getRes.status, 200);
		const m = await getRes.json();
		assertEquals(m.status, "Uploaded");
		assertEquals(m.size, data.length);
	});

	await t.step("Alice uploads via direct upload (async)", async () => {
		const aliceWs = new SyncClient(alice.token);
		await aliceWs.ready;

		try {
			const formData = new FormData();
			const data = "direct async upload data";
			formData.append(
				"file",
				new Blob([data], { type: "text/plain" }),
				"direct-async.txt",
			);
			formData.append(
				"json",
				new Blob([JSON.stringify({ async: true })], {
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
			assertEquals(res.status, 202);
			const { media_id } = await res.json();

			// Wait for MediaProcessed event
			const processedMsg = await aliceWs.waitFor((msg) =>
				msg.type === "MediaProcessed" && msg.media.id === media_id
			);
			assertEquals(processedMsg.media.size, data.length);
			assertEquals(processedMsg.media.status, "Uploaded");
		} finally {
			await aliceWs.close();
		}
	});
});
