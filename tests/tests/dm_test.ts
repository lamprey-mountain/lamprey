import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Direct Messages", async (t) => {
	const alice = await createTester("alice-dm");
	const bob = await createTester("bob-dm");
	const charlie = await createTester("charlie-dm");

	// Enable friend requests from everyone for Alice and Bob to simplify testing
	for (const t of [alice, bob]) {
		const prefs = await t({ url: "/preferences", status: 200 });
		prefs.privacy.friends.allow_everyone = true;
		await t({
			url: "/preferences",
			method: "PUT",
			body: prefs,
			status: 200,
		});
	}

	let channelId: string;

	await t.step("Alice starts a DM with Bob", async () => {
		await alice({
			url: `/user/@self/friend/${bob.user.id}`,
			method: "PUT",
			status: 204,
		});
		await bob({
			url: `/user/@self/friend/${alice.user.id}`,
			method: "PUT",
			status: 204,
		});

		const dm = await alice({
			url: `/user/@self/dm/${bob.user.id}`,
			method: "POST",
			status: 201,
		});
		console.log("DM initialized:", dm);
		assertEquals(dm.type, "Dm");
		channelId = dm.id;
	});

	await t.step("Alice sends a message in DM", async () => {
		const message = await alice({
			url: `/channel/${channelId}/message`,
			method: "POST",
			body: { content: "hello bob" },
			status: 201,
		});
		assertEquals(message.latest_version.content, "hello bob");
	});

	await t.step("Bob sees the message in DM", async () => {
		const messages = await bob({
			url: `/channel/${channelId}/message`,
			status: 200,
		});
		assertEquals(messages.items[0].latest_version.content, "hello bob");
	});

	await t.step("Charlie cannot see Alice and Bob's DM", async () => {
		await charlie({
			url: `/channel/${channelId}`,
			status: 404,
		});
	});

	await t.step("Alice lists her DMs", async () => {
		const dms = await alice({
			url: `/user/@self/dm`,
			status: 200,
		});
		assertEquals(dms.items.some((d: any) => d.id === channelId), true);
	});
});
