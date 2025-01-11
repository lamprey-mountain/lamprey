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

export const MessageList = createPagination({
	method: "get",
	path: "/api/v1/threads/{thread_id}/messages",
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
		201: {
			description: "edited",
			content: {
				"application/json": {
					schema: Message,
				},
			},
		},
		200: {
			description: "no change",
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

export const MessageVersionsList = createPagination({
	method: "get",
	path: "/api/v1/threads/{thread_id}/messages/{message_id}/version",
	summary: "Message versions list",
	tags: ["message"],
	request: {
		params: z.object({
			thread_id: MessageId,
		}),
	},
	pagination: {
		id: MessageId,
		ty: Message,
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
