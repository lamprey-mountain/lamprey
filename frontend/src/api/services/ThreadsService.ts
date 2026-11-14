import type { Channel, ThreadList } from "sdk";
import { createResource, type Resource } from "solid-js";
import { logger } from "@/utils/logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/threads");

const THREAD_PAGE_SIZE = 100;

type ThreadListType =
	| "room"
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
			["channel", new Map()],
			["channel_archived", new Map()],
			["channel_removed", new Map()],
		]);
	}

	getKey(item: Channel): string {
		return item.id;
	}

	async fetch(_id: string): Promise<Channel> {
		throw new Error("Use channels.fetch() for threads");
	}

	private getListMap(type: ThreadListType): Map<string, PaginatedList> {
		const map = this._roomLists.get(type);
		if (!map) {
			throw new Error(`Unknown thread list type: ${type}`);
		}
		return map;
	}

	private request(
		type: ThreadListType,
		identifier: string,
		cursor?: string,
	): Promise<{
		data?: ThreadList;
		error?: unknown;
		response: Response;
	}> {
		const query = { dir: "f", limit: THREAD_PAGE_SIZE, from: cursor } as const;

		switch (type) {
			case "room":
				return this.client.http.GET("/api/v1/room/{room_id}/thread", {
					params: { path: { room_id: identifier } },
				});
			case "channel":
				return this.client.http.GET("/api/v1/channel/{channel_id}/thread", {
					params: { path: { channel_id: identifier }, query },
				});
			case "channel_archived":
				return this.client.http.GET(
					"/api/v1/channel/{channel_id}/thread/archived",
					{ params: { path: { channel_id: identifier }, query } },
				);
			case "channel_removed":
				return this.client.http.GET(
					"/api/v1/channel/{channel_id}/thread/removed",
					{ params: { path: { channel_id: identifier }, query } },
				);
		}
	}

	private async fetchPage(
		list: PaginatedList,
		type: ThreadListType,
		identifier: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<ThreadList>(() =>
				this.request(type, identifier, list.state.cursor ?? undefined),
			);

			this.upsertBulk(data.threads);
			this.store.channels.upsertBulk(data.threads);

			const newIds = data.threads.map((thread) => thread.id);
			const hasMore =
				type !== "room" && data.threads.length >= THREAD_PAGE_SIZE;
			list.appendPage(
				newIds,
				hasMore,
				type === "room" ? undefined : (data.cursor ?? data.threads.at(-1)?.id),
			);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	private fetchNextPage(
		type: ThreadListType,
		identifier: string,
	): Promise<void> | undefined {
		const list = this.getListMap(type).get(identifier);
		if (!list) return;
		return this.fetchPage(list, type, identifier);
	}

	fetchNextChannelPage(channel_id: string): Promise<void> | undefined {
		return this.fetchNextPage("channel", channel_id);
	}

	fetchNextArchivedChannelPage(channel_id: string): Promise<void> | undefined {
		return this.fetchNextPage("channel_archived", channel_id);
	}

	fetchNextRemovedChannelPage(channel_id: string): Promise<void> | undefined {
		return this.fetchNextPage("channel_removed", channel_id);
	}

	private useList(
		id: () => string | undefined,
		type: ThreadListType,
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
				await this.fetchPage(list, type, identifier);
			}

			return list;
		});

		return resource;
	}

	useListForRoom(
		room_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(room_id, "room");
	}

	useListForChannel(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(channel_id, "channel");
	}

	useListArchivedForChannel(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(channel_id, "channel_archived");
	}

	useListRemovedForChannel(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		return this.useList(channel_id, "channel_removed");
	}

	handleThreadUpdate(thread: Channel) {
		const lists: {
			type: ThreadListType;
			room_id: string | null;
			channel_id: string | null;
			shouldInclude: boolean;
		}[] = [
			{
				type: "room",
				room_id: thread.room_id ?? null,
				channel_id: null,
				shouldInclude: !thread.archived_at && !thread.deleted_at,
			},
			{
				type: "channel",
				room_id: null,
				channel_id: thread.parent_id ?? null,
				shouldInclude: !thread.archived_at && !thread.deleted_at,
			},
			{
				type: "channel_archived",
				room_id: null,
				channel_id: thread.parent_id ?? null,
				shouldInclude: !!thread.archived_at,
			},
			{
				type: "channel_removed",
				room_id: null,
				channel_id: thread.parent_id ?? null,
				shouldInclude: !!thread.deleted_at,
			},
		];

		for (const listInfo of lists) {
			const id = listInfo.room_id ?? listInfo.channel_id;
			if (!id) continue;

			const map = this.getListMap(listInfo.type);
			const list = map.get(id);

			if (list) {
				if (listInfo.shouldInclude) {
					list.prependId(thread.id);
				} else {
					list.removeId(thread.id);
				}
			}
		}
	}

	clear() {
		super.clear();
		for (const listMap of this._roomLists.values()) {
			for (const list of listMap.values()) {
				list.clear();
			}
			listMap.clear();
		}
	}
}
