import { createTester, getSyncClient } from "../common.ts";
import { assertEquals } from "@std/assert";

Deno.test("member list syncing", async (t) => {
	const { tester: alice, user: aliceUser } = await createTester(
		"alice-members",
	);
	const { tester: bob, user: bobUser, token: bobToken } = await createTester(
		"bob-members",
	);
	const { tester: charlie, user: charlieUser } = await createTester(
		"charlie-members",
	);

	// Alice creates a room
	const room = await alice({
		url: "/room",
		method: "POST",
		body: { name: "member-list-test-room" },
		status: 201,
	});

	// Alice invites bob and charlie
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
	await charlie({
		url: `/invite/${invite.code}`,
		method: "POST",
		status: 204,
	});

	// Bob connects a sync client. He will be the only one online.
	const bobSync = await getSyncClient(bobToken);

	try {
		await t.step("initial room member list", async () => {
			bobSync.send({
				type: "MemberListSubscribe",
				room_id: room.id,
				ranges: [[0, 99]],
			});

			const syncMsg = await bobSync.waitFor((msg) =>
				msg.type === "MemberListSync" && msg.room_id === room.id &&
				msg.ops.some((op: any) => op.type === "Sync")
			);

			// Initial state: Bob is online, Alice and Charlie are offline.
			assertEquals(syncMsg.groups.length, 2);
			const onlineGroup = syncMsg.groups.find((g: any) => g.id === "online");
			const offlineGroup = syncMsg.groups.find((g: any) => g.id === "offline");
			assertEquals(onlineGroup.count, 1);
			assertEquals(offlineGroup.count, 2);

			const syncOp = syncMsg.ops.find((op: any) => op.type === "Sync");
			assertEquals(syncOp.users.length, 3);

			const names = syncOp.users.map((u: any) => u.name).sort();
			assertEquals(names, ["alice-members", "bob-members", "charlie-members"]);
		});

		await t.step("member joins room", async () => {
			const { tester: dave } = await createTester("dave-members");
			const invite = await alice({
				url: `/room/${room.id}/invite`,
				method: "POST",
				body: {},
				status: 201,
			});
			await dave({
				url: `/invite/${invite.code}`,
				method: "POST",
				status: 204,
			});

			const insertMsg = await bobSync.waitFor((msg) =>
				msg.type === "MemberListSync" &&
				msg.ops.some((op: any) =>
					op.type === "Insert" && op.user.name === "dave-members"
				)
			);

			// After Dave joins, there should be 3 offline members.
			const offlineGroup = insertMsg.groups.find((g: any) =>
				g.id === "offline"
			);
			assertEquals(offlineGroup.count, 3);
		});

		await t.step("hoisted role moves member", async () => {
			// Alice creates a hoisted role
			const role = await alice({
				url: `/room/${room.id}/role`,
				method: "POST",
				body: { name: "Hoisted", hoist: true, position: 1 },
				status: 201,
			});

			// Alice assigns the role to Charlie. Charlie is offline, so this should not move him yet.
			await alice({
				url: `/room/${room.id}/role/${role.id}/member/${charlieUser.id}`,
				method: "PUT",
				status: 200,
			});

			// Now assign the role to Bob, who is online. This should create a new group.
			await alice({
				url: `/room/${room.id}/role/${role.id}/member/${bobUser.id}`,
				method: "PUT",
				status: 200,
			});

			const insertMsg = await bobSync.waitFor((msg) =>
				msg.type === "MemberListSync" &&
				msg.ops.some((op: any) =>
					op.type === "Insert" && op.user.id === bobUser.id
				)
			);

			// A new group for the hoisted role should exist.
			const hoistedGroup = insertMsg.groups.find((g: any) =>
				typeof g.id === "object" && g.id.role === role.id
			);
			assertEquals(hoistedGroup.count, 1);

			// The online group should now be empty.
			const onlineGroup = insertMsg.groups.find((g: any) => g.id === "online");
			assertEquals(onlineGroup, undefined);
		});
	} finally {
		await bobSync.disconnect();
	}
});
