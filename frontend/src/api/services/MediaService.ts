import type { Media, User } from "sdk";
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

	async search(
		query: string,
	): Promise<{ media: Media[]; results: string[]; users: User[] }> {
		const data = await this.retryWithBackoff<{
			media: Media[];
			results: string[];
			users: User[];
		}>(() =>
			this.client.http.POST("/api/v1/media/search", {
				body: { query },
			}),
		);
		if (data.media) {
			for (const m of data.media) {
				this.upsert(m);
			}
		}
		if (data.users) {
			for (const user of data.users) {
				this.store.users.upsert(user);
			}
		}
		return data;
	}
}
