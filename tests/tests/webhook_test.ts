import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Webhooks", async (t) => {
	const alice = await createTester("alice-wh");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Webhook Test Room", public: true },
		status: 201,
	});
	const channel = await alice({
		url: `/room/${room.id}/channel`,
		method: "POST",
		body: { name: "webhooks", type: "Text" },
		status: 201,
	});
	const channelId = channel.id;

	let webhookId: string;
	let webhookToken: string;

	await t.step("Alice creates a webhook", async () => {
		const webhook = await alice({
			url: `/channel/${channelId}/webhook`,
			method: "POST",
			body: { name: "Captain Hook" },
			status: 201,
		});
		assertEquals(webhook.name, "Captain Hook");
		webhookId = webhook.id;
		webhookToken = webhook.token;
	});

	let messageId: string;

	await t.step("Execute the webhook", async () => {
		const message = await alice({
			url: `/webhook/${webhookId}/${webhookToken}`,
			method: "POST",
			body: { content: "ahoy from webhook!" },
			status: 201,
		});
		assertEquals(message.latest_version.content, "ahoy from webhook!");
		messageId = message.id;
	});

	await t.step("Get the webhook-sent message", async () => {
		const message = await alice({
			url: `/webhook/${webhookId}/${webhookToken}/message/${messageId}`,
			status: 200,
		});
		assertEquals(message.latest_version.content, "ahoy from webhook!");
	});

	await t.step("Edit the webhook message", async () => {
		const updated = await alice({
			url: `/webhook/${webhookId}/${webhookToken}/message/${messageId}`,
			method: "PATCH",
			body: { content: "updated ahoy!" },
			status: 200,
		});
		assertEquals(updated.latest_version.content, "updated ahoy!");
	});

	await t.step("Delete the webhook message", async () => {
		await alice({
			url: `/webhook/${webhookId}/${webhookToken}/message/${messageId}`,
			method: "DELETE",
			status: 204,
		});

		// Verify deletion
		await alice({
			url: `/webhook/${webhookId}/${webhookToken}/message/${messageId}`,
			status: 404,
		});
	});

	await t.step("Alice deletes the webhook", async () => {
		await alice({
			url: `/webhook/${webhookId}`,
			method: "DELETE",
			status: 204,
		});

		// Verify deletion
		await alice({
			url: `/webhook/${webhookId}`,
			status: 404,
		});
	});
});
