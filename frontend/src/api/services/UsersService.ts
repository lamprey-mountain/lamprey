import type {
	Pagination,
	Preferences,
	User,
	UserListFilter,
	UserWithRelationship,
} from "sdk";
import { type ListState, PaginatedList } from "../core/PaginatedList";
import { BaseService } from "../core/Service";

export class UsersService extends BaseService<UserWithRelationship> {
	protected cacheName = "user";

	/**
	 * all users on this server, paginated
	 *
	 * this is admin-only (requires the `MemberBan` permission on the server room)
	 * and includes users that are not present in the local cache (eg. users we
	 * have never interacted with)
	 */
	userList = new PaginatedList();

	getKey(item: User | UserWithRelationship): string {
		return item.id;
	}

	async fetch(id: string): Promise<UserWithRelationship> {
		return await this.retryWithBackoff<UserWithRelationship>(() =>
			this.client.http.GET("/api/v1/user/{user_id}", {
				params: { path: { user_id: id } },
			}),
		);
	}

	override upsert(user: User | UserWithRelationship) {
		const oldUser = this.cache.get(user.id);
		const updatedUser: UserWithRelationship = {
			...(oldUser ?? {
				relationship: {
					note: null,
					relation: null,
					petname: null,
				},
			}),
			...user,
		};
		super.upsert(updatedUser);

		// If this user is ourself, update the @self alias
		if (user.id === (this.store.session() as any)?.user_id) {
			this.cache.set("@self", updatedUser);
		}
	}

	async createUser(name: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/user", { body: { name } }),
		);
	}

	async fetchListAll(
		cursor?: string,
		filter?: UserListFilter,
	): Promise<Pagination<User>> {
		return await this.retryWithBackoff<Pagination<User>>(() =>
			this.client.http.GET("/api/v1/user", {
				params: {
					query: {
						dir: "f",
						limit: 100,
						from: cursor,
						filter,
					},
				},
			}),
		);
	}

	/** reset the paged user list, eg. when the filter changes */
	resetListAll(): void {
		this.userList.clear();
	}

	/** load the next page of all users on this server */
	async loadListAll(filter?: UserListFilter): Promise<ListState> {
		const list = this.userList;
		if (list.state.isLoading || !list.state.has_more) return list.state;

		list.setLoading(true);
		try {
			const data = await this.fetchListAll(
				list.state.cursor ?? undefined,
				filter,
			);
			// upsert individually to keep any existing relationship data
			for (const user of data.items) this.upsert(user);
			list.appendPage(
				data.items.map((user) => user.id),
				data.has_more,
				data.items.at(-1)?.id,
			);
			return list.state;
		} catch (e) {
			list.setError(e);
			throw e;
		}
	}

	async search(query: string): Promise<{ users: User[]; results: string[] }> {
		const data = await this.retryWithBackoff<{
			users: User[];
			results: string[];
		}>(() =>
			this.client.http.POST("/api/v1/user/search", {
				body: { query },
			}),
		);
		if (data.users) {
			for (const user of data.users) {
				this.upsert(user);
			}
		}
		return data;
	}

	async setPreferences(body: Preferences): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT("/api/v1/preferences", { body }),
		);
	}
}
