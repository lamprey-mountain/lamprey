import { createRoute, z } from "npm:@hono/zod-openapi";
import {
	Message,
	MessageId,
	MessagePatch,
	MessageVersionId,
	RoomId,
	ThreadId,
	Uint,
} from "../../types.ts";
import { common, createPagination } from "../common.ts";

export const MessageCreate = createRoute({
	method: "post",
	path: "/api/v1/threads/{thread_id}/messages",
	summary: "Message create",
	tags: ["message"],
	request: {
		params: z.object({
			thread_id: ThreadId,
		}),
		body: {
			content: {
				"application/json": {
					schema: MessagePatch,
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
					schema: Message,
				},
			},
		},
	},
});

export const MessageList2 = createPagination({
	method: "get",
	path: "/api/v2/threads/{thread_id}/messages",
	summary: "Message list",
	tags: ["message"],
	pagination: {
		id: MessageId,
		ty: Message,
	},
	request: {
		params: z.object({
			thread_id: ThreadId,
		}),
	},
	query: z.object({
		// pinned: z.boolean().optional(),
	}),
})

export const MessageList = createRoute({
	method: "get",
	path: "/api/v1/threads/{thread_id}/messages",
	summary: "Message list",
	tags: ["message"],
	request: {
		params: z.object({
			thread_id: ThreadId,
		}),
		query: z.object({
			after: MessageId.optional(),
			before: MessageId.optional(),
			// around: MessageId.optional(),
			// pinned: z.boolean().optional(),
			// limit: Uint.min(1).max(100).default(10),
			limit: z.string().default("10").transform((i) => parseInt(i, 10)).pipe(
				Uint.min(1).max(100),
			),
		}),
		// query: z.object({
		// 	from: MessageId.optional(),
		// 	to: MessageId.optional(),
		// 	dir: z.enum(["f", "b"]),
		// 	// pinned: z.boolean().optional(),
		// 	// limit: Uint.min(1).max(100).default(10),
		// 	limit: z.string().default("10").transform((i) => parseInt(i, 10)).pipe(
		// 		Uint.min(1).max(100),
		// 	),
		// }),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						items: Message.array(),
						total: Uint,
						has_more: z.boolean(),
					}),
				},
			},
		},
	},
});

export const MessageUpdate = createRoute({
	method: "patch",
	path: "/api/v1/threads/{thread_id}/messages/{message_id}",
	summary: "Message update",
	tags: ["message"],
	request: {
		params: z.object({
			message_id: MessageId,
			thread_id: ThreadId,
		}),
		body: {
			content: {
				"application/json": {
					schema: MessagePatch,
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
					schema: Message,
				},
			},
		},
	},
});

export const MessageDelete = createRoute({
	method: "delete",
	path: "/api/v1/threads/{thread_id}/messages/{message_id}",
	summary: "Message delete",
	tags: ["message"],
	request: {
		params: z.object({
			message_id: MessageId,
			thread_id: ThreadId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "success",
		},
	},
});

export const MessageGet = createRoute({
	method: "get",
	path: "/api/v1/threads/{thread_id}/messages/{message_id}",
	summary: "Message get",
	tags: ["message"],
	request: {
		params: z.object({
			message_id: MessageId,
			thread_id: ThreadId,
		}),
		query: z.object({
			after: MessageId.optional(),
			before: MessageId.optional(),
			// around: MessageId.optional(),
			// pinned: z.boolean().optional(),
			limit: Uint.min(1).max(100).default(10),
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						messages: Message.array(),
						total: Uint,
						has_more: z.boolean(),
					}),
				},
			},
		},
	},
});

export const MessageVersionsList = createRoute({
	method: "get",
	path: "/api/v1/threads/{thread_id}/messages/{message_id}",
	summary: "Message versions list",
	tags: ["message"],
	request: {
		params: z.object({
			thread_id: MessageId,
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Message,
				},
			},
		},
	},
});

export const MessageVersionsGet = createRoute({
	method: "get",
	path:
		"/api/v1/threads/{thread_id}/messages/{message_id}/version/{version_id}",
	summary: "Message versions get",
	tags: ["message"],
	request: {
		params: z.object({
			thread_id: ThreadId,
			message_id: MessageId,
			version_id: MessageVersionId,
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Message,
				},
			},
		},
	},
});

export const MessageVersionsDelete = createRoute({
	method: "delete",
	path:
		"/api/v1/threads/{thread_id}/message/{message_id}/version/{version_id}",
	summary: "Message versions delete",
	tags: ["message"],
	request: {
		params: z.object({
			thread_id: ThreadId,
			message_id: MessageId,
			version_id: MessageVersionId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "success",
		},
	},
});

export const MessageAck = createRoute({
	method: "put",
	path: "/api/v1/threads/{thread_id}/messages/{message_id}/ack",
	summary: "Message ack",
	tags: ["ack"],
	request: {
		params: z.object({
			message_id: MessageId,
			thread_id: ThreadId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "success",
		},
	},
});
