import { createRoute, z } from "npm:@hono/zod-openapi";
import { Role, RoleId, RolePatch, Room, RoomId } from "../../types.ts";
import { createPagination } from "../common.ts";

// export const RoleCreate = createRoute({
// 	method: "post",
// 	path: "/api/v1/rooms/{room_id}/roles",
// 	summary: "Role create",
// 	tags: ["role"],
// 	request: {
// 		params: z.object({
// 			room_id: RoomId,
// 		}),
// 		body: {
// 			content: {
// 				"application/json": {
// 					schema: RolePatch,
// 				},
// 			},
// 		},
// 	},
// 	responses: {
// 		201: {
// 			description: "created",
// 			content: {
// 				"application/json": {
// 					schema: Role,
// 				},
// 			},
// 		},
// 	},
// });

// export const RoleUpdate = createRoute({
// 	method: "patch",
// 	path: "/api/v1/rooms/{room_id}/roles/{role_id}",
// 	summary: "Role update",
// 	tags: ["role"],
// 	request: {
// 		params: z.object({
// 			room_id: RoomId,
// 			role_id: RoleId,
// 		}),
// 		body: {
// 			content: {
// 				"application/json": {
// 					schema: RolePatch,
// 				},
// 			},
// 		},
// 	},
// 	responses: {
// 		200: {
// 			description: "success",
// 			content: {
// 				"application/json": {
// 					schema: Role,
// 				},
// 			},
// 		},
// 	},
// });

// export const RoleDelete = createRoute({
// 	method: "delete",
// 	path: "/api/v1/rooms/{room_id}/roles/{role_id}",
// 	summary: "Role delete",
// 	tags: ["role"],
// 	request: {
// 		params: z.object({
// 			room_id: RoomId,
// 			role_id: RoleId,
// 		}),
// 		body: {
// 			content: {
// 				"application/json": {
// 					schema: RolePatch,
// 				},
// 			},
// 		},
// 	},
// 	responses: {
// 		204: {
// 			description: "success",
// 		},
// 	},
// });

// export const RoleGet = createRoute({
// 	method: "get",
// 	path: "/api/v1/rooms/{room_id}/roles/{role_id}",
// 	summary: "Role get",
// 	tags: ["role"],
// 	request: {
// 		params: z.object({
// 			room_id: RoomId,
// 			role_id: RoleId,
// 		}),
// 	},
// 	responses: {
// 		200: {
// 			description: "success",
// 			content: {
// 				"application/json": {
// 					schema: Role,
// 				},
// 			},
// 		},
// 	},
// });

export const RoleList = createPagination({
	method: "get",
	path: "/api/v1/rooms/{room_id}/roles",
	summary: "Role list",
	tags: ["role"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
	},
	pagination: {
	  id: RoomId,
	  ty: Role,
	},
});
