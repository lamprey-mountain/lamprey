import { createRoute, z } from "npm:@hono/zod-openapi";
import { Invite, InviteCode, InvitePatch, RoomId, UserId } from "../../types.ts";
import { createPagination } from "../common.ts";

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
		// TODO: invite response schema?
		// 200: {
		// 	description: "success",
		// 	content: {
		// 		// "application/json": {
		// 		//   schema: ,
		// 		// }
		// 	},
		// },
		204: {
			description: "success",
		},
	},
});

export const InviteRoomList = createPagination({
	method: "get",
	path: "/api/v1/rooms/{room_id}/invites",
	summary: "Invite room list",
	tags: ["invite"],
	request: {
  	params: z.object({
  		room_id: RoomId,
  	}),
	},
	pagination: {
	  id: InviteCode,
	  ty: Invite,
	},
});

export const InviteUserList = createPagination({
	method: "get",
	path: "/api/v1/users/{user_id}/invites",
	summary: "Invite user list",
	tags: ["invite"],
	request: {
  	params: z.object({
  		user_id: UserId,
  	}),
	},
	pagination: {
	  id: InviteCode,
	  ty: Invite,
	},
});

export const InviteUserListSelf = createPagination({
	method: "get",
	path: "/api/v1/users/@me/invites",
	summary: "Invite user list self",
	tags: ["invite"],
	pagination: {
	  id: InviteCode,
	  ty: Invite,
	},
});

export const InviteCreateRoom = createRoute({
	method: "post",
	path: "/api/v1/rooms/{room_id}/invites",
	summary: "Invite create room",
	tags: ["invite"],
	request: {
  	params: z.object({
  		room_id: RoomId,
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

export const InviteCreateUser = createRoute({
	method: "post",
	path: "/api/v1/users/{user_id}/invites",
	summary: "Invite create user",
	tags: ["invite"],
	request: {
  	params: z.object({
  		user_id: UserId,
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

export const InviteCreateUserSelf = createRoute({
	method: "post",
	path: "/api/v1/users/@me/invites",
	summary: "Invite create user",
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
