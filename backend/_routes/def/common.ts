import { createRoute, z } from "npm:@hono/zod-openapi";
import { Permissions } from "../data.ts";

export const common = {
	429: {
		description: "ratelimited",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	400: {
		description: "bad request",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	401: {
		description: "auth required",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	403: {
		description: "missing permissions",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
					// permissions: Permissions,
				}),
			},
		},
	},
	404: {
		description: "not found",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	500: {
		description: "internal error",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
};
