import { z, createRoute } from "npm:@hono/zod-openapi";
import { Media, MediaId } from "../types.ts";

export const MediaCreate = createRoute({
  method: "post",
  path: "/api/v1/media",
  summary: "Media create",
  description: "Create a new url to upload media to. Media should be PUT directly to the returned url, without any extra headers. Media not referenced/used in other api calls will be removed after a period of time.",
  tags: ["media"],
  request: {
    body: {
      content: {
        "application/json": {
          schema: Media.pick({
            filename: true,
            source_url: true,
            alt: true,
            size: true,
          }).extend({
            source_url: z.string().optional().describe("The source url to download this media from. The returned url will not be writable if this is specified."),
          }),
        }
      }
    },
  },
  responses: {
    200: {
      description: "success",
      content: {
        "application/json": {
          schema: z.object({
            media_id: MediaId,
            upload_url: z.string().url(),
          }),
        }
      }
    },
  }
});

export const MediaFinish = createRoute({
  method: "put",
  path: "/api/v1/media/{media_id}/finish",
  summary: "Media finish",
  description: "Finished uploading media, begin processing.",
  tags: ["media"],
  request: {
    params: z.object({
      media_id: MediaId
    }),
  },
  responses: {
    202: {
      description: "processing",
    },
  }
});

export const MediaGet = createRoute({
  method: "get",
  path: "/api/v1/media/{media_id}",
  summary: "Media get",
  description: "Get Media",
  tags: ["media"],
  request: {
    params: z.object({
      media_id: MediaId
    }),
  },
  responses: {
    200: {
      description: "success",
      content: {
        "application/json": {
          schema: Media,
        }
      }
    },
  }
});

export const MediaClone = createRoute({
  method: "post",
  path: "/api/v1/media/{media_id}/clone",
  summary: "Media clone",
  description: "Create a new unique piece of media from an existing id.",
  tags: ["media"],
  request: {
    params: z.object({
      media_id: MediaId
    }),
    body: {
      content: {
        "application/json": {
          schema: Media.pick({
            filename: true,
            alt: true,
          }),
        }
      }
    },
  },
  responses: {
    200: {
      description: "success",
      content: {
        "application/json": {
          schema: z.object({
            url: z.string().url(),
          }),
        }
      }
    },
  }
});
