import { Media } from "sdk";
import { BaseService } from "../core/Service";
import { fetchWithRetry } from "../util";

export class MediaService extends BaseService<Media> {
	getKey(item: Media): string {
		return item.id;
	}

	async fetch(id: string): Promise<Media> {
		return await fetchWithRetry(() =>
			this.client.http.GET("/api/v1/media/{media_id}", {
				params: { path: { media_id: id } },
			})
		);
	}
}
