import { createRoute, z } from "npm:@hono/zod-openapi";
import { Invite, InviteCode, InvitePatch, RoomId } from "../types.ts";

export const InviteCreate = createRoute({
	method: "post",
	path: "/api/v1/invites",
	summary: "Invite create",
	tags: ["invite"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: InvitePatch,
				},
			},
		},
	},
	responses: {
		201: {
			description: "created",
			content: {
				"application/json": {
					schema: Invite,
				},
			},
		},
	},
});

export const InviteDelete = createRoute({
	method: "delete",
	path: "/api/v1/invites/{invite_code}",
	summary: "Invite delete",
	tags: ["invite"],
	request: {
		params: z.object({
			invite_code: InviteCode,
		}),
		body: {
			content: {
				"application/json": {
					schema: InvitePatch,
				},
			},
		},
	},
	responses: {
		204: {
			description: "success",
		},
	},
});

export const InviteResolve = createRoute({
	method: "get",
	path: "/api/v1/invites/{invite_code}",
	summary: "Invite resolve",
	tags: ["invite"],
	request: {
		params: z.object({
			invite_code: InviteCode,
		}),
	},
	responses: {
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Invite,
				},
			},
		},
	},
});

export const InviteUse = createRoute({
	method: "post",
	path: "/api/v1/invites/{invite_code}",
	summary: "Invite use",
	tags: ["invite"],
	request: {
		params: z.object({
			invite_code: InviteCode,
		}),
	},
	responses: {
		200: {
			description: "success",
			content: {
				// TODO: schema
				// "application/json": {
				//   schema: ,
				// }
			},
		},
	},
});

export const InviteRoomList = createRoute({
	method: "get",
	path: "/api/v1/rooms/{room_id}/invites",
	summary: "Invite room list",
	tags: ["invite"],
	params: z.object({
		room_id: RoomId,
	}),
	responses: {
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						invites: Invite.array(),
					}),
				},
			},
		},
	},
});
