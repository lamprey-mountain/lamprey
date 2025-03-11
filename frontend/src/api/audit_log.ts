import type { AuditLogEntry, Pagination } from "sdk";
import { createEffect, createResource, type Resource } from "solid-js";
import type { Api } from "../api.tsx";

type Listing<T> = {
	pagination: Pagination<T> | null;
	prom: Promise<unknown> | null;
};

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
				"/api/v1/room/{room_id}/logs",
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

		const [resource, { mutate }] = createResource(
			room_id_signal,
			async (room_id) => {
				let l = this._cachedListings.get(room_id)!;
				if (l?.prom) {
					await l.prom;
					return l.pagination!;
				}

				if (!l) {
					l = {
						prom: null,
						pagination: null,
					};
					this._cachedListings.set(room_id, l);
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l!.pagination = res;
				l!.prom = null;

				for (const mut of this._listingMutators) {
					if (mut.room_id === room_id) mut.mutate(res);
				}

				return res!;
			},
		);

		const mut = { room_id: room_id_signal(), mutate };
		this._listingMutators.add(mut);

		createEffect(() => {
			mut.room_id = room_id_signal();
		});

		return resource;
	}
}
