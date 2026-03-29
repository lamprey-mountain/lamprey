import type { Invite, Pagination } from "sdk";
import { BaseService } from "../core/Service";
import { createMemo, createResource, onCleanup, type Resource } from "solid-js";
import { PaginatedList } from "../core/PaginatedList";
import { logger } from "../../logger";

const log = logger.for("api/invites");

export class InvitesService extends BaseService<Invite> {
	protected cacheName = "invite";

	private _roomLists = new Map<string, PaginatedList>();
	private _channelLists = new Map<string, PaginatedList>();
	private _serverList: PaginatedList | null = null;

	getKey(item: Invite): string {
		return item.code;
	}

	async fetch(id: string): Promise<Invite> {
		const data = await this.retryWithBackoff<Invite>(() =>
			this.client.http.GET("/api/v1/invite/{invite_code}", {
				params: { path: { invite_code: id } },
			})
		);
		this.upsert(data);
		return data;
	}

	async accept(invite_code: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/invite/{invite_code}", {
				params: { path: { invite_code } },
			})
		);
	}

	private async fetchRoomPage(
		room_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<Invite>>(() =>
				this.client.http.GET("/api/v1/room/{room_id}/invite", {
					params: {
						path: { room_id },
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				})
			);

			this.upsertBulk(data.items);

			const newCodes = data.items.map((invite) => invite.code);
			list.appendPage(newCodes, data.has_more, data.items.at(-1)?.code);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	private async fetchChannelPage(
		channel_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<Invite>>(() =>
				this.client.http.GET("/api/v1/channel/{channel_id}/invite", {
					params: {
						path: { channel_id },
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				})
			);

			this.upsertBulk(data.items);

			const newCodes = data.items.map((invite) => invite.code);
			list.appendPage(newCodes, data.has_more, data.items.at(-1)?.code);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	private async fetchServerPage(
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<Invite>>(() =>
				this.client.http.GET("/api/v1/server/invite", {
					params: {
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				})
			);

			this.upsertBulk(data.items);

			const newCodes = data.items.map((invite) => invite.code);
			list.appendPage(newCodes, data.has_more, data.items.at(-1)?.code);
		} catch (e) {
			log.error(String(e));
			list.setError(e);
			throw e;
		}
	}

	useRoomList(
		room_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(room_id, async (rid) => {
			if (!rid) return undefined;

			let list = this._roomLists.get(rid);
			if (!list) {
				list = new PaginatedList();
				this._roomLists.set(rid, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchRoomPage(rid, list);
			}

			return list;
		});

		return resource;
	}

	useChannelList(
		channel_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(channel_id, async (cid) => {
			if (!cid) return undefined;

			let list = this._channelLists.get(cid);
			if (!list) {
				list = new PaginatedList();
				this._channelLists.set(cid, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchChannelPage(cid, list);
			}

			return list;
		});

		return resource;
	}

	useServerList(): Resource<PaginatedList | undefined> {
		const [resource] = createResource(async () => {
			if (!this._serverList) {
				this._serverList = new PaginatedList();
			}

			if (
				this._serverList.state.ids.length === 0 &&
				!this._serverList.state.isLoading
			) {
				await this.fetchServerPage(this._serverList);
			}

			return this._serverList;
		});

		return resource;
	}
}
