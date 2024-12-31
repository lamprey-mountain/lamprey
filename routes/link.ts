import { z, createRoute } from "npm:@hono/zod-openapi";

export const LinkDiscord = createRoute({
  method: "get",
  path: "/api/v1/link/discord",
  summary: "Link to discord",
  tags: ["link"],
  request: {
    query: z.object({
      code: z.string(),
    }),
  },
  responses: {
    300: {
      description: "success",
    },
  }
});

