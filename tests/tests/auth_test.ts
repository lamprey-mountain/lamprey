
import { assertEquals } from "@std/assert";
import { createTester, BASE_URL } from "../common.ts";

Deno.test("User and Session Routes", async (t) => {
    const alice = await createTester("alice");

    await t.step("Get self user info", async () => {
        const user = await alice({ url: "/user/@self", status: 200 });
        assertEquals(user.name, "alice");
    });

    await t.step("Update self user info", async () => {
        const updated = await alice({
            url: "/user/@self",
            method: "PATCH",
            body: { name: "alice-renamed" },
            status: 200,
        });
        assertEquals(updated.name, "alice-renamed");
    });

    await t.step("Get session info", async () => {
        const session = await alice({ url: "/session/@self", status: 200 });
        assertEquals(session.user_id, alice.user.id);
    });

    await t.step("Update session name", async () => {
        const updated = await alice({
            url: "/session/@self",
            method: "PATCH",
            body: { name: "new-session-name" },
            status: 200,
        });
        assertEquals(updated.name, "new-session-name");
    });

    await t.step("List sessions", async () => {
        const sessions = await alice({ url: "/session", status: 200 });
        assertEquals(sessions.items.length >= 1, true);
    });
});
