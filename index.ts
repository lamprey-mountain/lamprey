import { logger } from "npm:hono/logger";
// import { nanoid } from "npm:nanoid";
import { OpenAPIHono } from "npm:@hono/zod-openapi";
import { apiReference } from "npm:@scalar/hono-api-reference";
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
app.openAPIRegistry.register("Permissions", t.Permissions);
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

app.doc("/api/docs.json", {
  openapi: "3.0.0",
  info: {
    version: "0.0.1",
    title: "My API",
  },
  // security: [{
  //   type: "apiKey",
  //   name: "authorization",
  //   in: "header",
  // }],
  tags: [
    {
      name: "room",
      description: "routes for messing with rooms. *mark* `down` [test](https://example.com)",
    },
  ],
  servers: [
    { url: "http://localhost:8000", description: "local dev" },
    { url: "https://chat.celery.eu.org", description: "production" },
  ],
});

app.get("/api/docs/*", apiReference({
  theme: "saturn",
  pageTitle: "api reference",
  pathRouting: { basePath: "/api/docs" },
  layout: "modern",
  // withDefaultFonts: false,
  hideClientButton: true,
  spec: { url: "/api/docs.json" },
}));

export default app;
