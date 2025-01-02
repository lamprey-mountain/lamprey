import { createRoute, z } from "npm:@hono/zod-openapi";
import {
	AuditLogEntry,
	AuditLogEntryId,
	Media,
	MediaId,
	Member,
	MemberId,
	Message,
	MessageId,
	Room,
	RoomId,
	Thread,
	ThreadId,
	Uint,
	User,
	UserId,
} from "../types.ts";

const Report = z.object({
	id: ReportId,
	creator: Member.or(User),
	item: Room.or(Member).or(User).or(Thread).or(Message).or(Media),
	reason: z.string(),
});

export const LogList = createRoute({
	method: "get",
	path: "/api/v1/rooms/{room_id}/log",
	summary: "Log list",
	tags: ["moderation"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
		query: z.object({
			pinned: z.boolean().optional(),
			limit: Uint.min(1).max(100).default(10),
			after: AuditLogEntryId.optional(),
			before: AuditLogEntryId.optional(),
			around: AuditLogEntryId.optional(),
		}),
	},
	responses: {
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						entries: AuditLogEntry.array(),
					}),
				},
			},
		},
	},
});

export const ReportCreateRoom = createRoute({
	method: "post",
	path: "/api/v1/rooms/{room_id}/report",
	summary: "Report create (room)",
	tags: ["moderation"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
		query: z.object({
			to: z.enum(["server"]),
		}),
		body: {
			content: {
				"application/json": {
					schema: z.object({
						reason: z.string(),
					}),
				},
			},
		},
	},
	responses: {
		201: {
			description: "success",
			content: {
				"application/json": {
					schema: Report,
				},
			},
		},
	},
});

export const ReportCreateUser = createRoute({
	method: "post",
	path: "/api/v1/users/{user_id}/report",
	summary: "Report create (user)",
	tags: ["moderation"],
	request: {
		params: z.object({
			room_id: RoomId,
		}),
		query: z.object({
			to: z.enum(["server"]),
		}),
		body: {
			content: {
				"application/json": {
					schema: z.object({
						reason: z.string(),
					}),
				},
			},
		},
	},
	responses: {
		201: {
			description: "success",
			content: {
				"application/json": {
					schema: Report,
				},
			},
		},
	},
});

export const ReportCreateThread = createRoute({
	method: "post",
	path: "/api/v1/threads/{thread_id}/report",
	summary: "Report create (thread)",
	tags: ["moderation"],
	request: {
		params: z.object({
			room_id: RoomId,
			thread_id: ThreadId,
		}),
		query: z.object({
			to: z.enum(["room", "server"]),
		}),
		body: {
			content: {
				"application/json": {
					schema: z.object({
						reason: z.string(),
					}),
				},
			},
		},
	},
	responses: {
		201: {
			description: "success",
			content: {
				"application/json": {
					schema: Report,
				},
			},
		},
	},
});

export const ReportCreateMessage = createRoute({
	method: "post",
	path:
		"/api/v1/threads/{thread_id}/messages/{message_id}/report",
	summary: "Report create (message)",
	tags: ["moderation"],
	request: {
		params: z.object({
			room_id: RoomId,
			thread_id: ThreadId,
			message_id: MessageId,
		}),
		query: z.object({
			to: z.enum(["room", "server"]),
		}),
		body: {
			content: {
				"application/json": {
					schema: z.object({
						reason: z.string(),
					}),
				},
			},
		},
	},
	responses: {
		201: {
			description: "success",
			content: {
				"application/json": {
					schema: Report,
				},
			},
		},
	},
});

export const ReportCreateMember = createRoute({
	method: "post",
	path: "/api/v1/rooms/{room_id}/members/{member_id}/report",
	summary: "Report create (member)",
	tags: ["moderation"],
	request: {
		params: z.object({
			room_id: RoomId,
			member_id: MemberId,
		}),
		query: z.object({
			to: z.enum(["room", "server"]),
		}),
		body: {
			content: {
				"application/json": {
					schema: z.object({
						reason: z.string(),
					}),
				},
			},
		},
	},
	responses: {
		201: {
			description: "success",
			content: {
				"application/json": {
					schema: Report,
				},
			},
		},
	},
});

export const ReportCreateMedia = createRoute({
	method: "post",
	path: "/api/v1/media/{media_id}/report",
	summary: "Report create (media)",
	tags: ["moderation"],
	request: {
		params: z.object({
			media_id: MediaId,
		}),
		query: z.object({
			to: z.enum(["server"]),
		}),
		body: {
			content: {
				"application/json": {
					schema: z.object({
						reason: z.string(),
					}),
				},
			},
		},
	},
	responses: {
		201: {
			description: "success",
			content: {
				"application/json": {
					schema: Report,
				},
			},
		},
	},
});
