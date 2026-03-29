import type { User, UserWithRelationship } from "sdk";
import { BaseService } from "../core/Service";

export class UsersService extends BaseService<UserWithRelationship> {
	protected cacheName = "user";

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

	async createGuest(name: string): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.POST("/api/v1/guest", { body: { name } }),
		);
	}

	async setPreferences(body: any): Promise<void> {
		await this.retryWithBackoff(() =>
			this.client.http.PUT("/api/v1/preferences", { body }),
		);
	}
}
