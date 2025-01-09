import { OpenAPIHono } from "@hono/zod-openapi";
import { events, HonoEnv } from "globals";
import setupRooms from "./rooms/index.ts";
import setupThreads from "./threads/index.ts";
import setupMessages from "./messages/index.ts";
import setupUsers from "./users/index.ts";
import setupSessions from "./sessions/index.ts";
import setupAuth from "./auth/index.ts";
import setupSync from "./sync/index.ts";
import setupMembers from "./members/index.ts";
import setupInvite from "./invite/index.ts";
import setupRoles from "./roles/index.ts";
import setupMedia from "./media/index.ts";
import setupDocs from "./docs.ts";

export function setup(app: OpenAPIHono<HonoEnv>) {
	setupRooms(app);
	setupThreads(app);
	setupMessages(app);
	setupUsers(app);
	setupSessions(app);
	setupAuth(app);
	setupSync(app);
	setupMembers(app);
	setupInvite(app);
	setupRoles(app);
	setupMedia(app);
	setupDocs(app);

	// stub for now, actually serve app later
	app.get("/", (c) => {
		return c.html(Deno.readTextFileSync("./index.html"));
	});

	
	const hooks = new Map([
		["da8eed9498f37713", "01943cc1-62e0-7c0e-bb9b-a4ff42864d69"],
	]);
	
	app.post("/api/v1/_temp_hooks/:id", async (c) => {
		const data = await c.req.json();
		const hook_id = c.req.param("id");
		const user_id = hooks.get(hook_id);
		if (!user_id) return c.json({ error: "not found" }, 404);
		events.emit("users", user_id, { type: "webhook", hook_id, data });
	  return new Response(null, { status: 204 });
	});
}
