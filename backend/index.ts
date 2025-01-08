import { logger } from "npm:hono/logger";
// import { nanoid } from "npm:nanoid";
import { OpenAPIHono } from "npm:@hono/zod-openapi";
import * as t from "./types.ts";
// import * as routes from "./routes.ts";
import * as routes from "./routes/index.ts";
import { HonoEnv } from "globals";
import { cors } from "hono/cors";

const app = new OpenAPIHono<HonoEnv>();

app.openAPIRegistry.register("Room", t.Room);
app.openAPIRegistry.register("Thread", t.Thread);
app.openAPIRegistry.register("Message", t.Message);
app.openAPIRegistry.register("Embed", t.Embed);
app.openAPIRegistry.register("Media", t.Media);
app.openAPIRegistry.register("User", t.User);
app.openAPIRegistry.register("Member", t.Member);
app.openAPIRegistry.register("Role", t.Role);
app.openAPIRegistry.register("Invite", t.Invite);
app.openAPIRegistry.register("Permission", t.Permission);
app.openAPIRegistry.registerComponent("securitySchemes", "token", {
	type: "apiKey",
	name: "authorization",
	in: "header",
});

app.use(logger());
app.use((c, next) => {
	// cors middleware has issues with websockets
	if (c.req.path === "/api/v1/sync") return next();
	return cors()(c, next);
});

routes.setup(app);

export default app;
