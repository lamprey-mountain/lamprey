
import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Audit Log Handling", async (t) => {
    const alice = await createTester("alice-audit");

    // Alice creates a room
    const room = await alice({
        url: "/room",
        method: "POST",
        body: { name: "Audit Test Room" },
        status: 201,
    });
    const roomId = room.id;

    await t.step("Update the room to generate audit log entry", async () => {
        await alice({
            url: `/room/${roomId}`,
            method: "PATCH",
            body: { name: "Renamed for Audit" },
            status: 200,
        });
    });

    await t.step("Check room audit log", async () => {
        const logs = await alice({
            url: `/room/${roomId}/audit-logs`,
            status: 200,
        });
        
        const roomUpdateEntry = logs.audit_log_entries.find((entry: any) => entry.type === "RoomUpdate");
        assertEquals(roomUpdateEntry.room_id, roomId);
        assertEquals(roomUpdateEntry.user_id, alice.user.id);
        assertEquals(roomUpdateEntry.metadata.changes[0].key, "name");
        assertEquals(roomUpdateEntry.metadata.changes[0].old, "Audit Test Room");
        assertEquals(roomUpdateEntry.metadata.changes[0].new, "Renamed for Audit");
    });

    await t.step("Check user audit log", async () => {
        await alice({
            url: "/user/@self",
            method: "PATCH",
            body: { name: "alice-renamed-audit" },
            status: 200,
        });

        const logs = await alice({
            url: "/user/@self/audit-logs",
            status: 200,
        });

        const userUpdateEntry = logs.audit_log_entries.find((entry: any) => entry.type === "UserUpdate");
        assertEquals(userUpdateEntry.user_id, alice.user.id);
    });
});
