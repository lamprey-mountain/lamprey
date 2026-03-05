
import { assertEquals } from "@std/assert";
import { createTester, SyncClient } from "../common.ts";

Deno.test("WebSocket Sync and Member List", async (t) => {
    const alice = await createTester("alice-sync");
    const bob = await createTester("bob-sync");

    const aliceWs = new SyncClient(alice.token);
    await aliceWs.ready;

    const bobWs = new SyncClient(bob.token);
    await bobWs.ready;

    // Alice creates a room
    const room = await alice({
        url: "/room",
        method: "POST",
        body: { name: "Sync Test Room" },
        status: 201,
    });
    const roomId = room.id;

    // Alice invites Bob
    const invite = await alice({
        url: `/room/${roomId}/invite`,
        method: "POST",
        body: {},
        status: 201,
    });
    const inviteCode = invite.code;

    // Bob joins via invite
    await bob({
        url: `/invite/${inviteCode}`,
        method: "POST",
        status: 204,
    });

    await t.step("Initial member list sync", async () => {
        aliceWs.send({
            type: "MemberListSubscribe",
            room_id: roomId,
            ranges: [[0, 99]],
        });

        const syncMsg = await aliceWs.waitFor((msg) =>
            msg.type === "MemberListSync" &&
            msg.room_id === roomId &&
            msg.ops.some((op: any) => op.type === "Sync")
        );

        // Should have Alice and Bob in the list
        const syncOp = syncMsg.ops.find((op: any) => op.type === "Sync");
        assertEquals(syncOp.items.length, 2);

        // Groups should reflect online status
        const onlineGroup = syncMsg.groups.find((g: any) => g.id === "Online");
        assertEquals(onlineGroup.count, 2);
    });

    await t.step("Bob presence update", async () => {
        await bob({
            url: "/user/@self/presence",
            method: "POST",
            body: { status: "Offline", activities: [] },
            status: 204,
        });

        // Wait for member list update reflecting Bob going offline
        const updateMsg = await aliceWs.waitFor((msg) =>
            msg.type === "MemberListSync" &&
            msg.room_id === roomId &&
            msg.ops.some((op: any) => op.type === "Delete" || op.type === "Insert"),
            10000
        );

        // After Bob goes offline, groups should be updated
        const onlineGroup = updateMsg.groups.find((g: any) => g.id === "Online");
        const offlineGroup = updateMsg.groups.find((g: any) => g.id === "Offline");
        assertEquals(onlineGroup.count, 1); // Alice
        assertEquals(offlineGroup.count, 1); // Bob
    });

    aliceWs.close();
    bobWs.close();
});
