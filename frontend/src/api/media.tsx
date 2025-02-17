import { Media } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { createEffect, createResource, Resource } from "solid-js";
import { Api } from "../api.tsx";

export class MediaInfo {
	api: Api = null as unknown as Api;
	cacheInfo = new ReactiveMap<string, Media>();
	_requests = new Map<string, Promise<Media>>();

	fetchInfo(media_id: () => string): Resource<Media> {
		const [resource, { mutate }] = createResource(
			media_id,
			(media_id) => {
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
					this._requests.get(media_id);
					this.cacheInfo.set(media_id, data);
					return data;
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
