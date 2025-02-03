import { User } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import { createEffect, createResource, Resource } from "solid-js";
import { Api } from "../api.tsx";

export class Users {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, User>();
	requests = new Map<string, Promise<User>>();

	fetch(user_id: () => string): Resource<User> {
		const [resource, { mutate }] = createResource(user_id, (user_id) => {
			const cached = this.cache.get(user_id);
			if (cached) return cached;
			const existing = this.requests.get(user_id);
			if (existing) return existing;

			const req = (async () => {
				const { data, error } = await this.api.client.http.GET(
					"/api/v1/user/{user_id}",
					{
						params: { path: { user_id } },
					},
				);
				if (error) throw error;
				this.requests.delete(user_id);
				this.cache.set(user_id, data);
				return data;
			})();

			createEffect(() => {
				mutate(this.cache.get(user_id));
			});

			this.requests.set(user_id, req);
			return req;
		});

		return resource;
	}
}
