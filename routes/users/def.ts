import { z, createRoute } from "npm:@hono/zod-openapi";
import { UserPatch, User, UserId } from "../../types.ts";
import { common } from "../common.ts";

export const UserCreate = createRoute({
  method: "post",
  path: "/api/v1/users",
  summary: "User create",
  tags: ["user"],
  request: {
    body: {
      content: {
        "application/json": {
          schema: UserPatch.required({ name: true }),
        }
      }
    }
  },
  responses: {
    ...common,
    201: {
      description: "success",
      content: {
        "application/json": {
          schema: User,
        }
      }
    },
  }
});

export const UserUpdate = createRoute({
  method: "patch",
  path: "/api/v1/users/{user_id}",
  summary: "User update",
  tags: ["user"],
  request: {
    params: z.object({
      user_id: UserId.or(z.literal("@me")),
    }),
    body: {
      content: {
        "application/json": {
          schema: UserPatch.pick({
            name: true,
            description: true,
            status: true,
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
          schema: User,
        }
      }
    },
  }
});

export const UserDelete = createRoute({
  method: "delete",
  path: "/api/v1/users/{user_id}",
  summary: "User delete",
  tags: ["user"],
  request: {
    params: z.object({
      user_id: UserId.or(z.literal("@me")),
    }),
    body: {
      content: {
        "application/json": {
          schema: UserPatch,
        }
      }
    }
  },
  responses: {
    ...common,
    204: {
      description: "success",
    },
  }
});

export const UserGet = createRoute({
  method: "get",
  path: "/api/v1/users/{user_id}",
  summary: "User get",
  tags: ["user"],
  request: {
    params: z.object({
      user_id: UserId.or(z.literal("@me")),
    }),
  },
  responses: {
    ...common,
    200: {
      description: "success",
      content: {
        "application/json": {
          schema: User,
        }
      }
    },
  }
});
