import type { Channel, Pagination } from "sdk";
import { createResource, type Resource } from "solid-js";
import { logger } from "../../logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/threads");

type ThreadListType =
	| "room"
	| "room_archived"
	| "room_removed"
	| "channel"
	| "channel_archived"
	| "channel_removed";

export class ThreadsService extends BaseService<Channel> {
	protected cacheName = "thread";

	private _roomLists = new Map<ThreadListType, Map<string, PaginatedList>>();

	constructor(...args: ConstructorParameters<typeof BaseService>) {
		super(...args);
		this._roomLists = new Map([
			["room", new Map()],
			["room_archived", new Map()],
			["room_removed", new Map()],
			["channel", new Map()],
			["channel_archived", new Map()],
			["channel_removed", new Map()],
		]);
	}

	getKey(item: Channel): string {
		return item.id;
	}

	async fetch(id: string): Promise<Channel> {
		throw new Error("Use channels.fetch() for threads");
	}

	private getListMap(type: ThreadListType): Map<string, PaginatedList> {
		return this._roomLists.get(type)!;
	}

	private async fetchPage(
		endpoint: string,
		pathParam: string,
		pathValue: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<Channel>>(() =>
				this.client.http.GET(endpoint as any, {
					params: {
						path: { [pathParam]: pathValue },
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				}),
			);

			this.upsertBulk(data.items);

			const newIds = data.items.map((thread) => thread.id);
			list.appendPage(newIds, data.has_more, data.items.at(-1)?.id);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	private useList(
		id: () => string | undefined,
		type: ThreadListType,
		endpoint: string,
		pathParam: string,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(id, async (identifier) => {
			if (!identifier) return undefined;

			const listMap = this.getListMap(type);
			let list = listMap.get(identifier);
			if (!list) {
				list = new PaginatedList();
				listMap.set(identifier, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchPage(endpoint, pathParam, identifier, list);
			}

			return list;
		});

		return resource;
	}

	useListForRoom(
		room_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(
			room_id,
			"room",
			"/api/v1/room/{room_id}/thread",
			"room_id",
		);
	}

	useListArchivedForRoom(
		room_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(
			room_id,
			"room_archived",
			"/api/v1/room/{room_id}/thread/archived",
			"room_id",
		);
	}

	useListRemovedForRoom(
		room_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(
			room_id,
			"room_removed",
			"/api/v1/room/{room_id}/thread/removed",
			"room_id",
		);
	}

	useListForChannel(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(
			channel_id,
			"channel",
			"/api/v1/channel/{channel_id}/thread",
			"channel_id",
		);
	}

	useListArchivedForChannel(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(
			channel_id,
			"channel_archived",
			"/api/v1/channel/{channel_id}/thread/archived",
			"channel_id",
		);
	}

	useListRemovedForChannel(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(
			channel_id,
			"channel_removed",
			"/api/v1/channel/{channel_id}/thread/removed",
			"channel_id",
		);
	}
}
