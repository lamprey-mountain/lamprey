import { createRoute, z } from "npm:@hono/zod-openapi";
import { Room, RoomId, RoomPatch, Uint, UserId } from "../../types.ts";
import { common } from "../common.ts";

export const RoomCreate = createRoute({
	method: "post",
	path: "/api/v1/rooms",
	summary: "Room create",
	tags: ["room"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: RoomPatch.required({ name: true }),
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
					schema: Room,
				},
			},
			links: {
				getRoom: {
					operationId: "room.get",
					parameters: {
						userId: "$response.body#/room_id",
					},
				},
			},
		},
	},
});

export const DmInitialize = createRoute({
	method: "put",
	path: "/api/v1/dm/{user_id}",
	summary: "Dm initialize",
	description: "Get or create a direct message room.",
	tags: ["room"],
	request: {
		params: z.object({
			user_id: UserId,
		}),
	},
	responses: {
		...common,
		201: {
			description: "created",
			content: {
				"application/json": {
					schema: Room,
				},
			},
		},
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Room,
				},
			},
		},
	},
});

export const DmGet = createRoute({
	method: "get",
	path: "/api/v1/dm/{user_id}",
	summary: "Dm get",
	description: "Get a direct message room.",
	tags: ["room"],
	request: {
		params: z.object({
			user_id: UserId,
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Room,
				},
			},
		},
	},
});

export const RoomList = createRoute({
	method: "get",
	path: "/api/v1/rooms",
	summary: "Room list",
	tags: ["room"],
	request: {
		query: z.object({
			after: RoomId.optional(),
			before: RoomId.optional(),
			around: RoomId.optional(),
			limit: z.string().default("10").transform((i) => parseInt(i, 10)).pipe(
				Uint.min(1).max(100).default(10),
			),
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						items: Room.array(),
						total: Uint,
						has_more: z.boolean(),
					}),
				},
			},
		},
	},
});

export const RoomUpdate = createRoute({
	method: "patch",
	path: "/api/v1/rooms/{room_id}",
	summary: "Room update",
	tags: ["room"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
		body: {
			content: {
				"application/json": {
					schema: RoomPatch,
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
					schema: Room,
				},
			},
		},
	},
});

export const RoomGet = createRoute({
	method: "get",
	path: "/api/v1/rooms/{room_id}",
	summary: "Room get",
	tags: ["room"],
	operationId: "room.get",
	request: {
		params: z.object({
			room_id: RoomId,
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Room,
				},
			},
		},
	},
});

export const RoomAck = createRoute({
	method: "put",
	path: "/api/v1/rooms/{room_id}/ack",
	summary: "Room ack",
	description: "Mark all threads in a room as read.",
	tags: ["ack"],
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
