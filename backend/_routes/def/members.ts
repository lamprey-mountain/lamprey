import { createRoute, z } from "npm:@hono/zod-openapi";
import { Member, MemberId, MemberPatch, RoleId, RoomId } from "../types.ts";

export const MemberList = createRoute({
	method: "get",
	path: "/api/v1/rooms/{room_id}/members",
	summary: "Member list",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
	},
	responses: {
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						members: Member.array(),
					}),
				},
			},
		},
	},
});

export const MemberGet = createRoute({
	method: "get",
	path: "/api/v1/rooms/{room_id}/members/{member_id}",
	summary: "Member get",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			member_id: MemberId,
		}),
	},
	responses: {
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

export const MemberUpdate = createRoute({
	method: "patch",
	path: "/api/v1/rooms/{room_id}/members/{member_id}",
	summary: "Member update",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			member_id: MemberId,
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

export const MemberKick = createRoute({
	method: "delete",
	path: "/api/v1/rooms/{room_id}/members/{member_id}",
	summary: "Member kick",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			member_id: MemberId,
		}),
	},
	responses: {
		204: {
			description: "success",
		},
	},
});

export const MemberRoleApply = createRoute({
	method: "put",
	path: "/api/v1/rooms/{room_id}/members/{member_id}/roles/{role_id}",
	summary: "Member role apply",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			member_id: MemberId,
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
	path: "/api/v1/rooms/{room_id}/members/{member_id}/roles/{role_id}",
	summary: "Member role remove",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
			member_id: MemberId,
			role_id: RoleId,
		}),
	},
	responses: {
		204: {
			description: "success",
		},
	},
});
