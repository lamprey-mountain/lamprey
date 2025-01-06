import { createRoute, z } from "npm:@hono/zod-openapi";
import { Member, MemberPatch, RoleId, RoomId, UserId } from "../../types.ts";
import { common, createPagination } from "../common.ts";

export const RoomMemberList = createPagination({
	method: "get",
	path: "/api/v1/rooms/{room_id}/members",
	summary: "Member list",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
	},
	pagination: {
	  id: UserId,
	  ty: Member,
	},
});

export const RoomMemberGet = createRoute({
	method: "get",
	path: "/api/v1/rooms/{room_id}/members/{user_id}",
	summary: "Member get",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			user_id: UserId.or(z.literal("@self")),
		}),
	},
	responses: {
	  ...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Member,
				},
			},
		},
	},
});

export const RoomMemberUpdate = createRoute({
	method: "patch",
	path: "/api/v1/rooms/{room_id}/members/{user_id}",
	summary: "Member update",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			user_id: UserId,
		}),
		body: {
			content: {
				"application/json": {
					schema: MemberPatch,
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
					schema: Member,
				},
			},
		},
	},
});

export const RoomMemberKick = createRoute({
	method: "delete",
	path: "/api/v1/rooms/{room_id}/members/{user_id}",
	summary: "Member kick",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			user_id: UserId.or(z.literal("@self")),
		}),
	},
	responses: {
	  ...common,
		204: {
			description: "success",
		},
	},
});

export const MemberRoleApply = createRoute({
	method: "put",
	path: "/api/v1/rooms/{room_id}/members/{user_id}/roles/{role_id}",
	summary: "Member role apply",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			user_id: UserId,
			role_id: RoleId,
		}),
	},
	responses: {
		204: {
			description: "success",
		},
	},
});

export const MemberRoleRemove = createRoute({
	method: "delete",
	path: "/api/v1/rooms/{room_id}/members/{user_id}/roles/{role_id}",
	summary: "Member role remove",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			user_id: UserId,
			role_id: RoleId,
		}),
	},
	responses: {
		204: {
			description: "success",
		},
	},
});
