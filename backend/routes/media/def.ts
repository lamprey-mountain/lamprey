import { createRoute, z } from "npm:@hono/zod-openapi";
import { Media, MediaCreateBody, MediaId, Uint } from "../../types.ts";
import { common } from "../common.ts";

const mediaUploadHeaders = z.object({
	"Upload-Offset": z.string().describe("how much is already uploaded"),
	"Upload-Length": z.string().describe("the total size of the upload"),
});

export const MediaCreate = createRoute({
	method: "post",
	path: "/api/v1/media",
	summary: "Media create",
	description:
		"Create a new url to upload media to. Use the media upload endpoint for actually uploading media. Media not referenced/used in other api calls will be removed after a period of time.",
	tags: ["media"],
	request: {
		body: {
			content: {
				"application/json": {
					schema: MediaCreateBody,
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
					schema: z.object({
						media_id: MediaId,
						upload_url: z.string().url().nullable(),
					}),
				},
			},
		},
	},
});

export const MediaUpload = createRoute({
	method: "patch",
	path: "/api/v1/media/{media_id}",
	summary: "Media upload",
	description: "Upload more data to the media.",
	tags: ["media"],
	request: {
		params: z.object({
			media_id: MediaId,
		}),
		body: {
			content: {
				"application/offset+octet-stream": {
					schema: {}
				},
			},
		},
		headers: z.object({
			"Content-Length": z.string(),
			"Upload-Offset": z.string(),
		}),
	},
	responses: {
		...common,
		200: {
			description: "upload done",
			content: {
				"application/json": {
					schema: Media,
				},
			},
			headers: mediaUploadHeaders,
		},
		204: {
			description: "upload appended",
			headers: mediaUploadHeaders,
		},
	},
});

export const MediaCheck = createRoute({
	method: "head",
	path: "/api/v1/media/{media_id}",
	summary: "Media check",
	description: "Get info about the upload.",
	tags: ["media"],
	request: {
		params: z.object({
			media_id: MediaId,
		}),
	},
	responses: {
		...common,
		204: {
			description: "metadata",
			headers: mediaUploadHeaders,
		},
	},
});

export const MediaGet = createRoute({
	method: "get",
	path: "/api/v1/media/{media_id}",
	summary: "Media get",
	description: "Get media (unrelated to media check)",
	tags: ["media"],
	request: {
		params: z.object({
			media_id: MediaId,
		}),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: Media,
				},
			},
		},
	},
});

export const MediaClone = createRoute({
	method: "post",
	path: "/api/v1/media/{media_id}/clone",
	summary: "Media clone",
	description: "Create a new unique piece of media from an existing id.",
	tags: ["media"],
	request: {
		params: z.object({
			media_id: MediaId,
		}),
		body: {
			content: {
				"application/json": {
					schema: Media.pick({
						filename: true,
						alt: true,
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
					schema: Media,
				},
			},
		},
	},
});
