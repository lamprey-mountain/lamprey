import { z, createRoute } from "npm:@hono/zod-openapi";
import { Uint, MessageId, MessagePatch, Message, RoomId, ThreadId, MessageVersionId } from "../types.ts";
import { common } from "./common.ts";

export const MessageCreate = createRoute({
  method: "post",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/messages",
  summary: "Message create",
  tags: ["message"],
  request: {
    params: z.object({
      room_id: RoomId,
      thread_id: ThreadId,
    }),
    body: {
      content: {
        "application/json": {
          schema: MessagePatch,
        }
      }
    }
  },
  responses: {
    ...common,
    201: {
      description: "created",
      content: {
        "application/json": {
          schema: Message,
        }
      }
    },
  }
});

export const MessageList = createRoute({
  method: "get",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/messages",
  summary: "Message list",
  tags: ["message"],
  request: {
    params: z.object({
      room_id: RoomId,
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
        }
      }
    },
  }
});

export const MessageUpdate = createRoute({
  method: "patch",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/messages/{message_id}",
  summary: "Message update",
  tags: ["message"],
  request: {
    params: z.object({
      message_id: MessageId,
      thread_id: ThreadId,
      room_id: RoomId,
    }),
    body: {
      content: {
        "application/json": {
          schema: MessagePatch,
        }
      }
    }
  },
  responses: {
    ...common,
    200: {
      description: "success",
      content: {
        "application/json": {
          schema: Message,
        }
      }
    },
  }
});

export const MessageDelete = createRoute({
  method: "delete",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/messages/{message_id}",
  summary: "Message delete",
  tags: ["message"],
  request: {
    params: z.object({
      message_id: MessageId,
      thread_id: ThreadId,
      room_id: RoomId,
    }),
  },
  responses: {
    ...common,
    204: {
      description: "success",
    },
  }
});

export const MessageGet = createRoute({
  method: "get",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/messages/{message_id}",
  summary: "Message get",
  tags: ["message"],
  request: {
    params: z.object({
      message_id: MessageId,
      thread_id: ThreadId,
      room_id: RoomId,
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
        }
      }
    },
  }
});

export const MessageVersionsList = createRoute({
  method: "get",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/messages/{message_id}",
  summary: "Message versions list",
  tags: ["message"],
  request: {
    params: z.object({
      room_id: RoomId,
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
        }
      }
    },
  }
});

export const MessageVersionsGet = createRoute({
  method: "get",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/messages/{message_id}/version/{version_id}",
  summary: "Message versions get",
  tags: ["message"],
  request: {
    params: z.object({
      room_id: RoomId,
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
        }
      }
    },
  }
});

export const MessageVersionsDelete = createRoute({
  method: "delete",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/message/{message_id}/version/{version_id}",
  summary: "Message versions delete",
  tags: ["message"],
  request: {
    params: z.object({
      room_id: RoomId,
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
  }
});

export const MessageAck = createRoute({
  method: "put",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/messages/{message_id}/ack",
  summary: "Message ack",
  tags: ["ack"],
  request: {
    params: z.object({
      message_id: MessageId,
      thread_id: ThreadId,
      room_id: RoomId,
    }),
  },
  responses: {
    ...common,
    204: {
      description: "success",
    },
  }
});
