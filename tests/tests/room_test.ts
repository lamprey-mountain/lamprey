
import { assertEquals } from "@std/assert";
import { createTester } from "../common.ts";

Deno.test("Room Routes and Basic Operations", async (t) => {
    const alice = await createTester("alice-room");

    let roomId: string;

    await t.step("Create a room", async () => {
        const room = await alice({
            url: "/room",
            method: "POST",
            body: { name: "Test Room", description: "A room for testing" },
            status: 201,
        });
        assertEquals(room.name, "Test Room");
        roomId = room.id;
    });

    await t.step("Get the created room", async () => {
        const room = await alice({ url: `/room/${roomId}`, status: 200 });
        assertEquals(room.id, roomId);
        assertEquals(room.name, "Test Room");
    });

    await t.step("Update the room", async () => {
        const updated = await alice({
            url: `/room/${roomId}`,
            method: "PATCH",
            body: { name: "Renamed Room" },
            status: 200,
        });
        assertEquals(updated.name, "Renamed Room");
    });

    await t.step("Delete the room", async () => {
        await alice({
            url: `/room/${roomId}`,
            method: "DELETE",
            status: 204,
        });
        
        // Should return 404 now
        await alice({ url: `/room/${roomId}`, status: 404 });
    });
});
