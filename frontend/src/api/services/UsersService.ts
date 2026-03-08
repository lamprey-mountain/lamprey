import { User, UserWithRelationship } from "sdk";
import { BaseService } from "../core/Service";
import { fetchWithRetry } from "../util";

export class UsersService extends BaseService<UserWithRelationship> {
	getKey(item: User | UserWithRelationship): string {
		return item.id;
	}

	async fetch(id: string): Promise<UserWithRelationship> {
		return await fetchWithRetry(() =>
			this.client.http.GET("/api/v1/user/{user_id}", {
				params: { path: { user_id: id } },
			})
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
					ignore: null,
				},
			}),
			...user,
		};
		super.upsert(updatedUser);

		// If this user is ourself, update the @self alias
		if (user.id === this.store.session()?.user_id) {
			this.cache.set("@self", updatedUser);
		}
	}
}
