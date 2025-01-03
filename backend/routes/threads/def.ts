import { createRoute, z } from "npm:@hono/zod-openapi";
import { RoomId, Thread, ThreadId, ThreadPatch, Uint } from "../../types.ts";
import { common, createPagination } from "../common.ts";

export const ThreadCreate = createRoute({
	method: "post",
	path: "/api/v1/rooms/{room_id}/threads",
	summary: "Thread create",
	tags: ["thread"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: ThreadPatch.required({ name: true }),
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
					schema: Thread,
				},
			},
		},
	},
});

export const ThreadList = createPagination({
	method: "get",
	path: "/api/v1/rooms/{room_id}/threads",
	summary: "Thread list",
	tags: ["thread"],
	pagination: {
		id: ThreadId,
		ty: Thread,
	},
	query: z.object({
		// pinned: z.boolean().optional(),
	}),
});

export const ThreadUpdate = createRoute({
	method: "patch",
	path: "/api/v1/threads/{thread_id}",
	summary: "Thread update",
	tags: ["thread"],
	request: {
		params: z.object({
			thread_id: ThreadId,
		}),
		body: {
			content: {
				"application/json": {
					schema: ThreadPatch,
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
					schema: Thread,
				},
			},
		},
	},
});

export const ThreadBulkUpdate = createRoute({
	method: "patch",
	path: "/api/v1/threads",
	summary: "Thread bulk update",
	tags: ["thread"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: z.object({
						threads: ThreadPatch.setKey("thread_id", ThreadId).array(),
					}),
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
					schema: z.object({
						threads: Thread.array(),
					}),
				},
			},
		},
	},
});

export const ThreadGet = createRoute({
	method: "get",
	path: "/api/v1/threads/{thread_id}",
	summary: "Thread get",
	tags: ["thread"],
	request: {
		params: z.object({
			thread_id: ThreadId,
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Thread,
				},
			},
		},
	},
});

export const ThreadAck = createRoute({
	method: "put",
	path: "/api/v1/threads/{thread_id}/ack",
	summary: "Thread ack",
	tags: ["ack"],
	request: {
		params: z.object({
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
