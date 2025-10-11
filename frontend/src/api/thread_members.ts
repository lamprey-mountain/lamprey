import type { Pagination, ThreadMember } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createEffect,
	createResource,
	type Resource,
	untrack,
} from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class ThreadMembers {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, ReactiveMap<string, ThreadMember>>();
	_requests = new Map<string, Map<string, Promise<ThreadMember>>>();
	_cachedListings = new Map<string, Listing<ThreadMember>>();

	subscribeList(thread_id: string, ranges: [number, number][]) {
		this.api.client.getWebsocket().send(JSON.stringify({
			type: "MemberListSubscribe",
			thread_id,
			ranges,
		}));
	}

	fetch(
		thread_id: () => string,
		user_id: () => string,
	): Resource<ThreadMember> {
		const query = () => ({
			thread_id: thread_id(),
			user_id: user_id(),
		});

		const [resource, { mutate }] = createResource(
			query,
			({ thread_id, user_id }) => {
				const cached = this.cache.get(thread_id)?.get(user_id);
				if (cached) return cached;
				const existing = this._requests.get(thread_id)?.get(user_id);
				if (existing) return existing;

				const req = (async () => {
					const { data, error } = await this.api.client.http.GET(
						"/api/v1/thread/{thread_id}/member/{user_id}",
						{
							params: { path: { thread_id, user_id } },
						},
					);
					// HACK: handle 404s
					type ErrorResp = { error: string } | undefined;
					if ((error as ErrorResp)?.error === "not found") {
						const placeholder: ThreadMember = {
							membership: "Leave",
							thread_id,
							user_id,
							membership_updated_at: new Date().toISOString(),
						};
						return placeholder;
					}
					if (error) throw error;
					this._requests.get(thread_id)?.delete(user_id);
					if (!this.cache.has(thread_id)) {
						this.cache.set(thread_id, new ReactiveMap());
					}
					this.cache.get(thread_id)!.set(user_id, data);
					return data;
				})();

				if (!this._requests.has(thread_id)) {
					this._requests.set(thread_id, new Map());
				}
				this._requests.get(thread_id)!.set(user_id, req);
				return req;
			},
		);

		createEffect(() => {
			const member = this.cache.get(thread_id())?.get(user_id());
			if (member) mutate(member);
		});

		return resource;
	}

	list(thread_id_sig: () => string): Resource<Pagination<ThreadMember>> {
		const thread_id = untrack(thread_id_sig);

		const paginate = async (pagination?: Pagination<ThreadMember>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/thread/{thread_id}/member",
				{
					params: {
						path: { thread_id: thread_id_sig() },
						query: {
							dir: "f",
							limit: 100,
							from: pagination?.items.at(-1)?.user_id,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			const thread_id = thread_id_sig();
			let cache = this.cache.get(thread_id);
			if (!cache) {
				cache = new ReactiveMap();
				this.cache.set(thread_id, cache);
			}

			batch(() => {
				for (const item of data.items) {
					cache.set(item.user_id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const l = this._cachedListings.get(thread_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2 = {
			resource: (() => {}) as unknown as Resource<Pagination<ThreadMember>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListings.set(thread_id, l2);

		const [resource, { refetch, mutate }] = createResource(
			thread_id_sig,
			async (thread_id) => {
				let l = this._cachedListings.get(thread_id)!;
				if (!l) {
					l = {
						resource: (() => {}) as unknown as Resource<
							Pagination<ThreadMember>
						>,
						refetch: () => {},
						mutate: () => {},
						prom: null,
						pagination: null,
					};
					this._cachedListings.set(thread_id, l);
				}
				if (l?.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l!.pagination = res;
				l!.prom = null;
				return res!;
			},
		);

		l2.resource = resource;
		l2.refetch = refetch;
		l2.mutate = mutate;

		return resource;
	}
}
