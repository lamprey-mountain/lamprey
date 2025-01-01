import { createRoute } from "npm:@hono/zod-openapi";

export const SyncInit = createRoute({
  method: "get",
  path: "/api/v1/sync",
  summary: "Sync init",
  tags: ["websocket"],
  responses: {
    101: {
      description: "upgrade to websocket",
    },
    500: {
      description: "error",
    },
  }
});

