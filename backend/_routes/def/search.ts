import { createRoute, z } from "npm:@hono/zod-openapi";
import {
	Message,
	MessageId,
	MessagePatch,
	MessageVersionId,
	RoomId,
	ThreadId,
	Uint,
	UserId,
} from "../types.ts";

// const a = (b: z.Schema) => b;
// a(MessageId);
// z.object({
//   query: z.string().openapi("SearchQuery"),
//   before: MessageId.optional(),
//   after: MessageId.optional(),
//   around: MessageId.optional(),
//   include: z.enum(["public", "muted"]).array(),
//   has: z.enum(["link", "file", "image", "video", "audio"]).array(),
//   sort: z.enum(["new", "old", "relevant", "irrelevant"]),
//   from: UserId.array().optional(),
//   mentions: UserId.array().optional(),
// })

export const SearchMessages = createRoute({
	method: "post",
	path: "/api/v1/search/messages",
	summary: "Search messages",
	tags: ["search"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: z.object({
						query: z.string(),
						author_ids: UserId.array().optional(),
						thread_ids: ThreadId.array().optional(),
						room_ids: RoomId.array().optional(),
						include_public: z.boolean(),
						include_muted: z.boolean(),
					}),
				},
			},
		},
	},
	responses: {
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						messages: Message.array(),
					}),
				},
			},
		},
	},
});

export const SearchThreads = createRoute({
	method: "post",
	path: "/api/v1/search/threads",
	summary: "Search threads",
	tags: ["search"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: z.object({
						query: z.string(),
						author_ids: UserId.array().optional(),
						room_ids: RoomId.array().optional(),
						include_public: z.boolean(),
						include_muted: z.boolean(),
					}),
				},
			},
		},
	},
	responses: {
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						threads: Message.array(),
					}),
				},
			},
		},
	},
});

export const SearchRooms = createRoute({
	method: "post",
	path: "/api/v1/search/rooms",
	summary: "Search rooms",
	tags: ["search"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: z.object({
						query: z.string(),
						include_public: z.boolean(),
						include_muted: z.boolean(),
					}),
				},
			},
		},
	},
	responses: {
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						rooms: Message.array(),
					}),
				},
			},
		},
	},
});
