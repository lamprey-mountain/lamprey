import type { Media } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { createEffect, createResource, type Resource } from "solid-js";
import type { Api } from "../api.tsx";

// V2 Media type (from backend)
type MediaV2 = {
	id: string;
	status: "Transferring" | "Processing" | "Uploaded" | "Consumed";
	filename: string;
	alt?: string | null;
	size: number;
	content_type: string;
	source_url?: string;
	metadata?: {
		type: "Image" | "Video" | "Audio" | "Text" | "File";
		width?: number;
		height?: number;
		duration?: number;
	};
	user_id?: string;
	deleted_at?: string;
	has_thumbnail: boolean;
	has_gifv: boolean;
	[K: string]: any;
};

/** Convert V2 media format to V1 format for backwards compatibility */
export function convertV2MediaToV1(media: MediaV2): Media {
	let trackInfo: any = {};
	let mime = media.content_type || "application/octet-stream";

	if (media.metadata) {
		switch (media.metadata.type) {
			case "Image":
				trackInfo = {
					type: "Image",
					width: media.metadata.width || 0,
					height: media.metadata.height || 0,
					language: null,
				};
				break;
			case "Video":
				trackInfo = {
					type: "Mixed",
					width: media.metadata.width || null,
					height: media.metadata.height || null,
					duration: media.metadata.duration || null,
					language: null,
				};
				break;
			case "Audio":
				trackInfo = {
					type: "Mixed",
					width: null,
					height: null,
					duration: media.metadata.duration || null,
					language: null,
				};
				break;
			case "Text":
				trackInfo = {
					type: "Text",
					language: null,
				};
				break;
			default:
				trackInfo = { type: "Other" };
		}
	} else {
		trackInfo = { type: "Other" };
	}

	return {
		id: media.id,
		filename: media.filename,
		alt: media.alt ?? null,
		source: {
			info: trackInfo,
			size: media.size,
			mime: mime,
			source: media.source_url ? "Downloaded" : "Uploaded",
			source_url: media.source_url,
		},
	};
}

export class MediaInfo {
	api: Api = null as unknown as Api;
	cacheInfo = new ReactiveMap<string, Media>();
	_requests = new Map<string, Promise<Media>>();

	fetchInfo(media_id: () => string): Resource<Media> {
		const [resource, { mutate }] = createResource(
			media_id,
			async (media_id) => {
				const cached = this.cacheInfo.get(media_id);
				if (cached) return cached;
				const existing = this._requests.get(media_id);
				if (existing) return existing;

				const req = (async () => {
					const { data, error } = await this.api.client.http.GET(
						"/api/v1/media/{media_id}",
						{
							params: { path: { media_id } },
						},
					);
					if (error) throw error;
					this._requests.delete(media_id);
					const converted = "status" in data
						? convertV2MediaToV1(data as MediaV2)
						: data as Media;
					this.cacheInfo.set(media_id, converted);
					return converted;
				})();

				this._requests.set(media_id, req);
				return req;
			},
		);

		createEffect(() => {
			const media = this.cacheInfo.get(media_id());
			if (media) mutate(media);
		});

		return resource;
	}
}
