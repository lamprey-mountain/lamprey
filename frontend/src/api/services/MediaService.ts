import type { Media } from "sdk";
import { BaseService } from "../core/Service";

export class MediaService extends BaseService<Media> {
	protected cacheName = "media";

	getKey(item: Media): string {
		return item.id;
	}

	async fetch(id: string): Promise<Media> {
		return await this.retryWithBackoff<Media>(() =>
			this.client.http.GET("/api/v1/media/{media_id}", {
				params: { path: { media_id: id } },
			}),
		);
	}
}
