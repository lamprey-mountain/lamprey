import { createRoute, z } from "npm:@hono/zod-openapi";
import { UserId } from "../types.ts";

export const ServerInfo = createRoute({
	method: "get",
	path: "/api/v1/info",
	summary: "Server info",
	tags: ["core"],
	responses: {
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						version: z.string(),
						user_id: UserId.optional(),
					}),
				},
			},
		},
	},
});
