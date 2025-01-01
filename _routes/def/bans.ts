import { z, createRoute } from "npm:@hono/zod-openapi";
import { RoomId, UserId } from "../types.ts";

export const BanCreate = createRoute({
  method: "put",
  path: "/api/v1/rooms/{room_id}/bans/{user_id}",
  summary: "Ban create",
  tags: ["member"],
  request: {
    params: z.object({
      room_id: RoomId,
      user_id: UserId,
    }),
  },
  responses: {
    204: {
      description: "success",
    },
  }
});

export const BanDelete = createRoute({
  method: "delete",
  path: "/api/v1/rooms/{room_id}/bans/{user_id}",
  summary: "Ban remove",
  tags: ["member"],
  request: {
    params: z.object({
      room_id: RoomId,
      user_id: UserId,
    }),
  },
  responses: {
    204: {
      description: "success",
    },
  }
});
