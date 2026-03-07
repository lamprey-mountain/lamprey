import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Message Reactions", async (t) => {
	const alice = await createTester("alice-re");
	const bob = await createTester("bob-re");

	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "Reaction Test Room", public: true },
		status: 201,
	});

	// Bob joins the room
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

	const channel = await alice({
		url: `/room/${room.id}/channel`,
		method: "POST",
		body: { name: "general", type: "Text" },
		status: 201,
	});

	const message = await alice({
		url: `/channel/${channel.id}/message`,
		method: "POST",
		body: { content: "react to me" },
		status: 201,
	});
	const messageId = message.id;

	await t.step("Alice adds a reaction", async () => {
		const reactionKey = "t:👍";
		await alice({
			url: `/channel/${channel.id}/message/${messageId}/reaction/` +
				encodeURIComponent(reactionKey) + "/@self",
			method: "PUT",
			status: 200,
		});

		const msg = await alice({
			url: `/channel/${channel.id}/message/${messageId}`,
			status: 200,
		});
		const reaction = msg.reactions.find((r: any) => r.key.content === "👍");
		assertEquals(reaction.count, 1);
		assertEquals(reaction.self, true);
	});

	await t.step("Bob adds the same reaction", async () => {
		const reactionKey = "t:👍";
		await bob({
			url: `/channel/${channel.id}/message/${messageId}/reaction/` +
				encodeURIComponent(reactionKey) + "/@self",
			method: "PUT",
			status: 200,
		});

		const msg = await bob({
			url: `/channel/${channel.id}/message/${messageId}`,
			status: 200,
		});
		const reaction = msg.reactions.find((r: any) => r.key.content === "👍");
		assertEquals(reaction.count, 2);
		assertEquals(reaction.self, true);
	});

	await t.step("Listing users for a reaction", async () => {
		const reactionKey = "t:👍";
		const reactors = await alice({
			url: `/channel/${channel.id}/message/${messageId}/reaction/` +
				encodeURIComponent(reactionKey),
			status: 200,
		});
		assertEquals(reactors.items.length, 2);
	});

	await t.step("Alice removes her reaction", async () => {
		const reactionKey = "t:👍";
		await alice({
			url: `/channel/${channel.id}/message/${messageId}/reaction/` +
				encodeURIComponent(reactionKey) + "/@self",
			method: "DELETE",
			status: 204,
		});

		const msg = await alice({
			url: `/channel/${channel.id}/message/${messageId}`,
			status: 200,
		});
		const reaction = msg.reactions.find((r: any) => r.key.content === "👍");
		assertEquals(reaction.count, 1);
		assertEquals(reaction.self, false);
	});
});
