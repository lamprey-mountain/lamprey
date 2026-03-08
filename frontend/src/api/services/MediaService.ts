import { Media } from "sdk";
import { BaseService } from "../core/Service";
import { fetchWithRetry } from "../util";

export class MediaService extends BaseService<Media> {
	async fetch(id: string): Promise<Media> {
		return await fetchWithRetry(() =>
			this.client.http.GET("/api/v1/media/{media_id}", {
				params: { path: { media_id: id } },
			})
		);
	}
}
