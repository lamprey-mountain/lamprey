import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("User Relationships (Friends, Blocking, Ignoring)", async (t) => {
	const alice = await createTester("alice-rel");
	const bob = await createTester("bob-rel");
	const charlie = await createTester("charlie-rel");

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

	await t.step("Alice sends friend request to Bob", async () => {
		await alice({
			url: `/user/@self/friend/${bob.user.id}`,
			method: "PUT",
			status: 204,
		});

		// Check Alice's pending list
		const alicePending = await alice({
			url: `/user/${alice.user.id}/friend/pending`,
			status: 200,
		});
		const outgoing = alicePending.items.find((r: any) =>
			r.user_id === bob.user.id
		);
		assertEquals(outgoing.relation, "Outgoing");

		// Check Bob's pending list
		const bobPending = await bob({
			url: `/user/${bob.user.id}/friend/pending`,
			status: 200,
		});
		const incoming = bobPending.items.find((r: any) =>
			r.user_id === alice.user.id
		);
		assertEquals(incoming.relation, "Incoming");
	});

	await t.step("Bob accepts Alice's friend request", async () => {
		await bob({
			url: `/user/@self/friend/${alice.user.id}`,
			method: "PUT",
			status: 204,
		});

		// Check Alice's friend list
		const aliceFriends = await alice({
			url: `/user/${alice.user.id}/friend`,
			status: 200,
		});
		assertEquals(
			aliceFriends.items.some((r: any) => r.user_id === bob.user.id),
			true,
		);

		// Check Bob's friend list
		const bobFriends = await bob({
			url: `/user/${bob.user.id}/friend`,
			status: 200,
		});
		assertEquals(
			bobFriends.items.some((r: any) => r.user_id === alice.user.id),
			true,
		);
	});

	await t.step("Alice removes Bob from friends", async () => {
		await alice({
			url: `/user/@self/friend/${bob.user.id}`,
			method: "DELETE",
			status: 204,
		});

		const aliceFriends = await alice({
			url: `/user/${alice.user.id}/friend`,
			status: 200,
		});
		assertEquals(
			aliceFriends.items.some((r: any) => r.user_id === bob.user.id),
			false,
		);
	});

	await t.step("Alice blocks Charlie", async () => {
		await alice({
			url: `/user/@self/block/${charlie.user.id}`,
			method: "PUT",
			status: 204,
		});

		const aliceBlocks = await alice({
			url: `/user/${alice.user.id}/block`,
			status: 200,
		});
		assertEquals(
			aliceBlocks.items.some((r: any) => r.user_id === charlie.user.id),
			true,
		);
	});

	await t.step("Alice unblocks Charlie", async () => {
		await alice({
			url: `/user/@self/block/${charlie.user.id}`,
			method: "DELETE",
			status: 204,
		});

		const aliceBlocks = await alice({
			url: `/user/${alice.user.id}/block`,
			status: 200,
		});
		assertEquals(
			aliceBlocks.items.some((r: any) => r.user_id === charlie.user.id),
			false,
		);
	});

	await t.step("Alice ignores Bob", async () => {
		const until = new Date(Date.now() + 1000 * 60 * 60).toISOString();
		await alice({
			url: `/user/@self/ignore/${bob.user.id}`,
			method: "PUT",
			body: {
				until,
			},
			status: 204,
		});

		const aliceIgnores = await alice({
			url: `/user/${alice.user.id}/ignore`,
			status: 200,
		});
		const ignore = aliceIgnores.items.find((r: any) =>
			r.user_id === bob.user.id
		);
		assertEquals(!!ignore.until, true);
	});

	await t.step("Alice unignores Bob", async () => {
		await alice({
			url: `/user/@self/ignore/${bob.user.id}`,
			method: "DELETE",
			status: 204,
		});

		const aliceIgnores = await alice({
			url: `/user/${alice.user.id}/ignore`,
			status: 200,
		});
		assertEquals(
			aliceIgnores.items.some((r: any) => r.user_id === bob.user.id),
			false,
		);
	});
});
