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
		const reactionKey = "👍";
		await alice({
			url:
				`/channel/${channel.id}/message/${messageId}/reaction/${reactionKey}/@self`,
			method: "PUT",
			status: 200,
		});

		const msg = await alice({
			url: `/channel/${channel.id}/message/${messageId}`,
			status: 200,
		});
		const reaction = msg.reactions.find((r: any) => r.key.content === "👍");
		assertEquals(reaction.count, 1);
		assertEquals(reaction.me, true);
	});

	await t.step("Bob adds the same reaction", async () => {
		const reactionKey = "👍";
		await bob({
			url:
				`/channel/${channel.id}/message/${messageId}/reaction/${reactionKey}/@self`,
			method: "PUT",
			status: 200,
		});

		const msg = await bob({
			url: `/channel/${channel.id}/message/${messageId}`,
			status: 200,
		});
		const reaction = msg.reactions.find((r: any) => r.key.content === "👍");
		assertEquals(reaction.count, 2);
		assertEquals(reaction.me, true);
	});

	await t.step("Listing users for a reaction", async () => {
		const reactionKey = "👍";
		const reactors = await alice({
			url:
				`/channel/${channel.id}/message/${messageId}/reaction/${reactionKey}`,
			status: 200,
		});
		assertEquals(reactors.items.length, 2);
	});

	await t.step("Alice removes her reaction", async () => {
		const reactionKey = "👍";
		await alice({
			url:
				`/channel/${channel.id}/message/${messageId}/reaction/${reactionKey}/@self`,
			method: "DELETE",
			status: 204,
		});

		const msg = await alice({
			url: `/channel/${channel.id}/message/${messageId}`,
			status: 200,
		});
		const reaction = msg.reactions.find((r: any) => r.key.content === "👍");
		assertEquals(reaction.count, 1);
		assertEquals(reaction.me, false);
	});
});
