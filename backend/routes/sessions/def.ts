import { createRoute, z } from "npm:@hono/zod-openapi";
import { Room, RoomId, Session, SessionId, SessionPatch } from "../../types.ts";
import { common } from "../common.ts";

export const SessionCreate = createRoute({
	method: "post",
	path: "/api/v1/sessions",
	summary: "Session create",
	tags: ["sessions"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: SessionPatch.required({ user_id: true }),
				},
			},
		},
	},
	responses: {
		...common,
		201: {
			description: "created",
			content: {
				"application/json": {
					schema: Session,
				},
			},
		},
	},
});

export const SessionUpdate = createRoute({
	method: "patch",
	path: "/api/v1/sessions/{session_id}",
	summary: "Session update",
	tags: ["sessions"],
	request: {
		params: z.object({
			session_id: SessionId.or(z.literal("@me")),
		}),
		body: {
			content: {
				"application/json": {
					schema: SessionPatch.pick({ name: true }),
				},
			},
		},
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Session,
				},
			},
		},
	},
});

export const SessionDelete = createRoute({
	method: "delete",
	path: "/api/v1/sessions/{session_id}",
	summary: "Session delete",
	tags: ["sessions"],
	request: {
		params: z.object({
			session_id: SessionId.or(z.literal("@me")).or(z.literal("@all")),
		}),
		body: {
			content: {
				"application/json": {
					schema: SessionPatch,
				},
			},
		},
	},
	responses: {
		...common,
		204: {
			description: "success",
		},
	},
});

export const SessionGet = createRoute({
	method: "get",
	path: "/api/v1/sessions/{session_id}",
	summary: "Session get",
	tags: ["sessions"],
	request: {
		params: z.object({
			session_id: SessionId.or(z.literal("@me")),
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Session,
				},
			},
		},
	},
});

// const createRouteHelp = (
//   method: "get" | "post" | "put" | "patch" | "delete",
//   path: string,
//   config: {
//     summary: string,
//     description: string,
//     tags: string[],
//   },
// ) => createRoute({
//   method,
//   path,
//   summary: config.summary,
//   description: config.description,
//   tags: config.tags,
//   responses: {
//     200: {
//       description: "success",
//       content: {
//         "application/json": {
//           schema: z.object({
//             sessions: Session.array(),
//           }),
//         }
//       }
//     },
//   }
// });

export const SessionList = createRoute({
	method: "get",
	path: "/api/v1/sessions",
	summary: "Session list",
	tags: ["sessions"],
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						sessions: Session.array(),
					}),
				},
			},
		},
	},
});
