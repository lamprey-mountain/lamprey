import { ThreadMember } from "sdk";
import { BaseService } from "../core/Service";
import { Accessor, createResource, Resource } from "solid-js";

export class ThreadMembersService extends BaseService<ThreadMember> {
	protected cacheName = "thread_member";

	getKey(item: ThreadMember): string {
		return `${item.thread_id}:${item.user_id}`;
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

	useMember(
		thread_id: Accessor<string>,
		user_id: Accessor<string>,
	): Resource<ThreadMember | undefined> {
		const id = () => {
			const t = thread_id();
			const u = user_id();
			return t && u
				? this.getKey({ thread_id: t, user_id: u } as any)
				: undefined;
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
}
