import { z, createRoute } from "npm:@hono/zod-openapi";

const SendEvents = z.union([
  z.object({
    op: z.literal("ping"),
    data: z.literal("foobar"),
  }),
  z.object({
    op: z.literal("hello"),
    data: z.literal("something"),
  }),
]);

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
