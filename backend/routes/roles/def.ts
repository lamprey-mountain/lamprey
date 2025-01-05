import { createRoute, z } from "npm:@hono/zod-openapi";
import { Member, Role, RoleId, RolePatch, Room, RoomId, UserId } from "../../types.ts";
import { common, createPagination } from "../common.ts";

export const RoleCreate = createRoute({
	method: "post",
	path: "/api/v1/rooms/{room_id}/roles",
	summary: "Role create",
	tags: ["role"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
		body: {
			content: {
				"application/json": {
					schema: RolePatch,
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
					schema: Role,
				},
			},
		},
	},
});

export const RoleUpdate = createRoute({
	method: "patch",
	path: "/api/v1/rooms/{room_id}/roles/{role_id}",
	summary: "Role update",
	tags: ["role"],
	request: {
		params: z.object({
			room_id: RoomId,
			role_id: RoleId,
		}),
		body: {
			content: {
				"application/json": {
					schema: RolePatch,
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
					schema: Role,
				},
			},
		},
	},
});

export const RoleDelete = createRoute({
	method: "delete",
	path: "/api/v1/rooms/{room_id}/roles/{role_id}",
	summary: "Role delete",
	tags: ["role"],
	request: {
		params: z.object({
			room_id: RoomId,
			role_id: RoleId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "success",
		},
	},
});

export const RoleGet = createRoute({
	method: "get",
	path: "/api/v1/rooms/{room_id}/roles/{role_id}",
	summary: "Role get",
	tags: ["role"],
	request: {
		params: z.object({
			room_id: RoomId,
			role_id: RoleId,
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Role,
				},
			},
		},
	},
});

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

export const RoleListMembers = createPagination({
	method: "get",
	path: "/api/v1/rooms/{room_id}/roles/{role_id}",
	summary: "Role list members",
	description: "Get a list of members with this role",
	tags: ["role"],
	request: {
		params: z.object({
			room_id: RoomId,
			role_id: RoleId,
		}),
	},
	pagination: {
		id: UserId,
		ty: Member,
	},
});

export const ThreadOverwriteRoleSet = createRoute({
	method: "put",
	path: "/api/v1/threads/{thread_id}/overwrites/roles/{role_id}",
	summary: "Thread overwrite role set",
	tags: ["overwrite"],
	request: {
		params: z.object({
			room_id: RoomId,
			role_id: RoleId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "created",
		},
	},
});

export const ThreadOverwriteRoleDelete = createRoute({
	method: "delete",
	path: "/api/v1/threads/{thread_id}/overwrites/roles/{role_id}",
	summary: "Thread overwrite role set",
	tags: ["overwrite"],
	request: {
		params: z.object({
			room_id: RoomId,
			role_id: RoleId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "created",
		},
	},
});

export const ThreadOverwriteUserSet = createRoute({
	method: "put",
	path: "/api/v1/threads/{thread_id}/overwrites/users/{user_id}",
	summary: "Thread overwrite user set",
	tags: ["overwrite"],
	request: {
		params: z.object({
			room_id: RoomId,
			user_id: UserId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "created",
		},
	},
});

export const ThreadOverwriteUserDelete = createRoute({
	method: "delete",
	path: "/api/v1/threads/{thread_id}/overwrites/users/{user_id}",
	summary: "Thread overwrite user set",
	tags: ["overwrite"],
	request: {
		params: z.object({
			room_id: RoomId,
			user_id: UserId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "created",
		},
	},
});
