import { z, createRoute } from "npm:@hono/zod-openapi";
import { Uint, ThreadId, ThreadPatch, Thread, RoomId } from "../types.ts";
import { common } from "./common.ts";

export const ThreadCreate = createRoute({
  method: "post",
  path: "/api/v1/rooms/{room_id}/threads",
  summary: "Thread create",
  tags: ["thread"],
  request: {
    params: z.object({
      room_id: RoomId,
    }),
    body: {
      content: {
        "application/json": {
          schema: ThreadPatch.required({ name: true }),
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
          schema: Thread,
        }
      }
    },
  }
});

export const ThreadList = createRoute({
  method: "get",
  path: "/api/v1/rooms/{room_id}/threads",
  summary: "Thread list",
  tags: ["thread"],
  request: {
    params: z.object({
      room_id: RoomId,
    }),
    query: z.object({
      // pinned: z.boolean().optional(),
      limit: Uint.min(1).max(100).default(10),
      after: ThreadId.optional(),
      before: ThreadId.optional(),
      // around: ThreadId.optional(),
    }),
  },
  responses: {
    ...common,
    200: {
      description: "success",
      content: {
        "application/json": {
          schema: z.object({
            rooms: Thread.array(),
            total: Uint,
            has_more: z.boolean(),
          }),
        }
      }
    },
  }
});

export const ThreadUpdate = createRoute({
  method: "patch",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}",
  summary: "Thread update",
  tags: ["thread"],
  request: {
    params: z.object({
      thread_id: ThreadId,
      room_id: RoomId,
    }),
    body: {
      content: {
        "application/json": {
          schema: ThreadPatch,
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
          schema: Thread,
        }
      }
    },
  }
});

export const ThreadBulkUpdate = createRoute({
  method: "patch",
  path: "/api/v1/rooms/{room_id}/threads",
  summary: "Thread bulk update",
  tags: ["thread"],
  request: {
    params: z.object({
      room_id: RoomId,
    }),
    body: {
      content: {
        "application/json": {
          schema: z.object({
            threads: ThreadPatch.setKey("thread_id", ThreadId).array(),
          }),
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
          schema: z.object({
            threads: Thread.array(),
          }),
        }
      }
    },
  }
});

export const ThreadGet = createRoute({
  method: "get",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}",
  summary: "Thread get",
  tags: ["thread"],
  request: {
    params: z.object({
      room_id: RoomId,
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
        }
      }
    },
  }
});

export const ThreadAck = createRoute({
  method: "put",
  path: "/api/v1/rooms/{room_id}/threads/{thread_id}/ack",
  summary: "Thread ack",
  tags: ["ack"],
  request: {
    params: z.object({
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
