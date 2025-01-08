import { OpenAPIHono } from "@hono/zod-openapi";
import { HonoEnv } from "globals";
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
	setupDocs(app);

	// stub for now, actually serve app later
	app.get("/", (c) => {
		return c.html(Deno.readTextFileSync("./index.html"));
	});
}
