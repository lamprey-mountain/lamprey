import type { InviteWithMetadata, Pagination } from "sdk";
import { createResource, type Resource } from "solid-js";
import { logger } from "@/utils/logger";
import { PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

const log = logger.for("api/invites");

type InviteCreateBody = {
	description?: string | null;
	expires_at?: string | null;
	max_uses?: number | null;
	role_ids?: string[] | null;
};

export class InvitesService extends BaseService<InviteWithMetadata> {
	protected cacheName = "invite";

	private _roomLists = new Map<string, PaginatedList>();
	private _channelLists = new Map<string, PaginatedList>();
	private _serverList: PaginatedList | null = null;

	getKey(item: InviteWithMetadata): string {
		return item.code;
	}

	async fetch(id: string): Promise<InviteWithMetadata> {
		const invite = await this.retryWithBackoff<InviteWithMetadata>(() =>
			this.client.http.GET("/api/v1/invite/{invite_code}", {
				params: { path: { invite_code: id } },
			}),
		);
		this.upsert(invite);
		return invite;
	}

	async accept(invite_code: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/invite/{invite_code}", {
				params: { path: { invite_code } },
			}),
		);
	}

	private async createInvite(
		params: { room_id: string } | { channel_id: string },
		body: InviteCreateBody,
	): Promise<InviteWithMetadata> {
		const invite = await this.retryWithBackoff<InviteWithMetadata>(() =>
			"room_id" in params
				? this.client.http.POST("/api/v1/room/{room_id}/invite", {
						params: { path: { room_id: params.room_id } },
						body,
					})
				: this.client.http.POST("/api/v1/channel/{channel_id}/invite", {
						params: { path: { channel_id: params.channel_id } },
						body,
					}),
		);

		this.upsert(invite);
		this.prependToLists(invite);
		return invite;
	}

	createRoomInvite(room_id: string, body: InviteCreateBody = {}) {
		return this.createInvite({ room_id }, body);
	}

	createChannelInvite(channel_id: string, body: InviteCreateBody = {}) {
		return this.createInvite({ channel_id }, body);
	}

	private prependToLists(invite: InviteWithMetadata) {
		const target = invite.target;
		if (target.type === "Room") {
			this._roomLists.get(target.room.id)?.prependId(invite.code);
			if (target.channel?.id) {
				this._channelLists.get(target.channel.id)?.prependId(invite.code);
			}
		} else if (target.type === "Gdm") {
			this._channelLists.get(target.channel.id)?.prependId(invite.code);
		} else if (target.type === "Server") {
			this._serverList?.prependId(invite.code);
		}
	}

	async deleteInvite(code: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.DELETE("/api/v1/invite/{invite_code}", {
				params: { path: { invite_code: code } },
			}),
		);
		this.delete(code);
	}

	protected override afterDelete(id: string) {
		for (const list of this._roomLists.values()) list.removeId(id);
		for (const list of this._channelLists.values()) list.removeId(id);
		this._serverList?.removeId(id);
	}

	fetchNextRoomPage(room_id: string): Promise<void> | undefined {
		const list = this._roomLists.get(room_id);
		if (!list) return;
		return this.fetchRoomPage(room_id, list, list.state.cursor ?? undefined);
	}

	fetchNextChannelPage(channel_id: string): Promise<void> | undefined {
		const list = this._channelLists.get(channel_id);
		if (!list) return;
		return this.fetchChannelPage(
			channel_id,
			list,
			list.state.cursor ?? undefined,
		);
	}

	fetchNextServerPage(): Promise<void> | undefined {
		const list = this._serverList;
		if (!list) return;
		return this.fetchServerPage(list, list.state.cursor ?? undefined);
	}

	private async fetchRoomPage(
		room_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<Pagination<InviteWithMetadata>>(
				() =>
					this.client.http.GET("/api/v1/room/{room_id}/invite", {
						params: {
							path: { room_id },
							query: {
								dir: "f",
								limit: 100,
								from: cursor,
							},
						},
					}),
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
			const data = await this.retryWithBackoff<Pagination<InviteWithMetadata>>(
				() =>
					this.client.http.GET("/api/v1/channel/{channel_id}/invite", {
						params: {
							path: { channel_id },
							query: {
								dir: "f",
								limit: 100,
								from: cursor,
							},
						},
					}),
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
			const data = await this.retryWithBackoff<Pagination<InviteWithMetadata>>(
				() =>
					this.client.http.GET("/api/v1/server/invite", {
						params: {
							query: {
								dir: "f",
								limit: 100,
								from: cursor,
							},
						},
					}),
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

	clear() {
		super.clear();
		for (const list of this._roomLists.values()) {
			list.clear();
		}
		this._roomLists.clear();
		for (const list of this._channelLists.values()) {
			list.clear();
		}
		this._channelLists.clear();
		if (this._serverList) {
			this._serverList.clear();
		}
	}
}
