import type { ThreadMember } from "sdk";
import { BaseService } from "../core/Service";
import { type Accessor, createResource, type Resource } from "solid-js";
import { PaginatedList } from "../core/PaginatedList";
import { logger } from "../../logger";

export class ThreadMembersService extends BaseService<ThreadMember> {
	protected cacheName = "thread_member";

	private _threadLists = new Map<string, PaginatedList>();

	getKey(item: ThreadMember): string {
		return `${item.thread_id}:${item.user_id}`;
	}

	private compositeId(thread_id: string, user_id: string): string {
		return `${thread_id}:${user_id}`;
	}

	protected getDbKey(id: string): IDBValidKey {
		const [thread_id, user_id] = id.split(":");
		return [thread_id, user_id];
	}

	override upsert(item: ThreadMember) {
		this.cache.set(this.getKey(item), item);

		if (this.db && this.cacheName) {
			this.db.put(this.cacheName, item).catch((e) => {
				console.warn(`Failed to write to ${this.cacheName}`, {
					key: [item.thread_id, item.user_id],
					error: e,
				});
			});
		}
	}

	async fetch(id: string): Promise<ThreadMember> {
		// id is "thread_id:user_id"
		const [thread_id, user_id] = id.split(":");
		if (!thread_id || !user_id) throw new Error("Invalid composite ID");

		try {
			const data = await this.retryWithBackoff<ThreadMember>(() =>
				this.client.http.GET("/api/v1/thread/{thread_id}/member/{user_id}", {
					params: { path: { thread_id, user_id } },
				})
			);
			return data;
		} catch (error: any) {
			if (error?.error === "not found") {
				// Placeholder
				return {
					membership: "Leave" as any,
					thread_id,
					user_id,
					joined_at: new Date().toISOString(),
				};
			}
			throw error;
		}
	}

	// TODO: rename to useThreadMember
	useMember(
		thread_id: Accessor<string>,
		user_id: Accessor<string>,
	): Resource<ThreadMember | undefined> {
		const id = () => {
			const t = thread_id();
			const u = user_id();
			return t && u ? this.compositeId(t, u) : undefined;
		};
		return this.use(id);
	}

	subscribeList(thread_id: string, ranges: [number, number][]) {
		this.client.send({
			type: "MemberListSubscribe",
			thread_id,
			ranges,
		});
	}

	private async fetchThreadPage(
		thread_id: string,
		list: PaginatedList,
		cursor?: string,
	): Promise<void> {
		if (list.state.isLoading || !list.state.has_more) return;
		list.setLoading(true);

		try {
			const data = await this.retryWithBackoff<
				{ items: ThreadMember[]; has_more: boolean }
			>(() =>
				this.client.http.GET("/api/v1/thread/{thread_id}/member", {
					params: {
						path: { thread_id },
						query: {
							dir: "f",
							limit: 100,
							from: cursor,
						},
					},
				})
			);

			this.upsertBulk(data.items);

			const newIds = data.items.map((member) => this.getKey(member));
			list.appendPage(newIds, data.has_more, data.items.at(-1)?.user_id);
		} catch (e) {
			logger.for("api/thread_members").error(String(e));
			list.setError(e);
			throw e;
		}
	}

	useList(
		thread_id: () => string | undefined,
	): Resource<PaginatedList | undefined> {
		const [resource] = createResource(thread_id, async (tid) => {
			if (!tid) return undefined;

			let list = this._threadLists.get(tid);
			if (!list) {
				list = new PaginatedList();
				this._threadLists.set(tid, list);
			}

			if (list.state.ids.length === 0 && !list.state.isLoading) {
				await this.fetchThreadPage(tid, list);
			}

			return list;
		});

		return resource;
	}
}
