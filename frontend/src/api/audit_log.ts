import type { AuditLogEntry, Pagination } from "sdk";
import { createEffect, createResource, type Resource, untrack } from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class AuditLogs {
	api: Api = null as unknown as Api;
	_cachedListings = new Map<string, Listing<AuditLogEntry>>();
	_listingMutators = new Set<
		{ room_id: string; mutate: (value: Pagination<AuditLogEntry>) => void }
	>();

	fetch(room_id_signal: () => string): Resource<Pagination<AuditLogEntry>> {
		const paginate = async (pagination?: Pagination<AuditLogEntry>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/room/{room_id}/audit-logs",
				{
					params: {
						path: { room_id: room_id_signal() },
						query: {
							dir: "b",
							limit: 100,
							from: pagination?.items.at(-1)?.id,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items.toReversed()],
			};
		};

		const room_id = untrack(room_id_signal);
		const l = this._cachedListings.get(room_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2 = {
			resource: (() => {}) as unknown as Resource<Pagination<AuditLogEntry>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListings.set(room_id, l2);

		const [resource, { refetch, mutate }] = createResource(
			room_id_signal,
			async (room_id) => {
				const l = this._cachedListings.get(room_id)!;
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
