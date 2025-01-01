import { createRoute, z } from "npm:@hono/zod-openapi";
import { Room, RoomId, Session, User, UserId, UserPatch } from "../../types.ts";
import { common } from "../common.ts";

// export const AuthLogin = createRoute({
//   method: "put",
//   path: "/api/v1/__temp_login",
//   summary: "Auth login (TEMP)",
//   tags: ["auth"],
//   request: {
//     body: {
//       content: {
//         "application/json": {
//           schema: z.object({
//             email: z.string(),
//             password: z.string()
//           }),
//         }
//       }
//     }
//   },
//   responses: {
//     ...common,
//     201: {
//       description: "created",
//       content: {
//         "application/json": {
//           schema: Session,
//         }
//       },
//     },
//   }
// });

export const AuthPasswordSet = createRoute({
	method: "put",
	path: "/api/v1/users/@me/password",
	summary: "Auth password set",
	tags: ["auth"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: z.object({
						password: z.string(),
					}),
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

export const AuthPasswordDo = createRoute({
	method: "post",
	path: "/api/v1/users/@me/password",
	summary: "Auth password do",
	tags: ["auth"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: z.object({
						password: z.string(),
					}),
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

export const AuthTotpSet = createRoute({
	method: "put",
	path: "/api/v1/users/@me/totp",
	summary: "Auth totp set",
	tags: ["auth"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: z.object({
						enable: z.boolean(),
					}),
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

export const AuthTotpDo = createRoute({
	method: "post",
	path: "/api/v1/users/@me/totp",
	summary: "Auth totp do",
	tags: ["auth"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: z.object({
						code: z.string().regex(/^[0-9]{6}$/),
					}),
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

export const AuthDiscordStart = createRoute({
	method: "get",
	path: "/api/v1/auth/discord",
	summary: "Auth discord info",
	tags: ["auth"],
	responses: {
		...common,
		302: {
			description: "success",
			headers: z.object({
				Location: z.string().url(),
			}),
		},
	},
});

export const AuthDiscordFinish = createRoute({
	method: "get",
	path: "/api/v1/auth/discord/redirect",
	summary: "Auth discord finish",
	tags: ["auth"],
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"text/html": {
					schema: z.string(),
				},
			},
		},
	},
});

export const AuthDiscordLogout = createRoute({
	method: "get",
	path: "/api/v1/auth/discord/logout",
	summary: "Auth discord logout",
	tags: ["auth"],
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"text/plain": {
					schema: z.string(),
				},
			},
		},
	},
});
