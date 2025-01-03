import { createRoute, z } from "npm:@hono/zod-openapi";
import { Member, MemberId, MemberPatch, RoleId, RoomId } from "../../types.ts";
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
	  id: MemberId,
	  ty: Member,
	},
});

export const RoomMemberGet = createRoute({
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

export const RoomMemberLeave = createRoute({
	method: "delete",
	path: "/api/v1/rooms/{room_id}/members/@self",
	summary: "Member kick self",
	description: "Leave room",
	tags: ["member"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
	},
	responses: {
	  ...common,
		204: {
			description: "success",
		},
	},
});

export const RoomMemberKick = createRoute({
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
	  ...common,
		204: {
			description: "success",
		},
	},
});

// export const MemberRoleApply = createRoute({
// 	method: "put",
// 	path: "/api/v1/rooms/{room_id}/members/{member_id}/roles/{role_id}",
// 	summary: "Member role apply",
// 	tags: ["member"],
// 	request: {
// 		params: z.object({
// 			room_id: RoomId,
// 			member_id: MemberId,
// 			role_id: RoleId,
// 		}),
// 	},
// 	responses: {
// 		204: {
// 			description: "success",
// 		},
// 	},
// });

// export const MemberRoleRemove = createRoute({
// 	method: "delete",
// 	path: "/api/v1/rooms/{room_id}/members/{member_id}/roles/{role_id}",
// 	summary: "Member role remove",
// 	tags: ["member"],
// 	request: {
// 		params: z.object({
// 			room_id: RoomId,
// 			member_id: MemberId,
// 			role_id: RoleId,
// 		}),
// 	},
// 	responses: {
// 		204: {
// 			description: "success",
// 		},
// 	},
// });
