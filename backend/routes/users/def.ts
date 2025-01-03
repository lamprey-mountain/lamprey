import { createRoute, z } from "npm:@hono/zod-openapi";
import { User, UserId, UserPatch } from "../../types.ts";
import { common } from "../common.ts";

export const UserCreate = createRoute({
	method: "post",
	path: "/api/v1/users",
	summary: "User create",
	tags: ["user"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: UserPatch.required({ name: true }),
				},
			},
		},
	},
	responses: {
		...common,
		201: {
			description: "success",
			content: {
				"application/json": {
					schema: User,
				},
			},
		},
	},
});

export const UserUpdate = createRoute({
	method: "patch",
	path: "/api/v1/users/{user_id}",
	summary: "User update",
	tags: ["user"],
	request: {
		params: z.object({
			user_id: UserId,
		}),
		body: {
			content: {
				"application/json": {
					schema: UserPatch.pick({
						name: true,
						description: true,
						status: true,
					}),
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
					schema: User,
				},
			},
		},
	},
});

export const UserUpdateSelf = createRoute({
	method: "patch",
	path: "/api/v1/users/@self",
	summary: "User update self",
	tags: ["user"],
	request: {
		params: z.object({
			user_id: z.literal("@self"),
		}),
		body: {
			content: {
				"application/json": {
					schema: UserPatch.pick({
						name: true,
						description: true,
						status: true,
					}),
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
					schema: User,
				},
			},
		},
	},
});

export const UserDelete = createRoute({
	method: "delete",
	path: "/api/v1/users/{user_id}",
	summary: "User delete",
	tags: ["user"],
	request: {
		params: z.object({
			user_id: UserId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "success",
		},
	},
});

export const UserDeleteSelf = createRoute({
	method: "delete",
	path: "/api/v1/users/@self",
	summary: "User delete self",
	tags: ["user"],
	responses: {
		...common,
		204: {
			description: "success",
		},
	},
});

export const UserGet = createRoute({
	method: "get",
	path: "/api/v1/users/{user_id}",
	summary: "User get",
	tags: ["user"],
	request: {
		params: z.object({
			user_id: UserId.or(z.literal("@self")),
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: User,
				},
			},
		},
	},
});

// export const UserGetSelf = createRoute({
// 	method: "get",
// 	path: "/api/v1/users/@self",
// 	summary: "User get self",
// 	tags: ["user"],
// 	responses: {
// 		...common,
// 		200: {
// 			description: "success",
// 			content: {
// 				"application/json": {
// 					schema: User,
// 				},
// 			},
// 		},
// 	},
// });
