import type { api } from "../index";
import type { Uuid } from "../core/uuid";

export type ApiUser = api["User"];

export type User = Omit<ApiUser, "id"> & UserExt;
export type UserId = Uuid & { readonly __type: "User" };

export type UserExt = {
	id: UserId;
};

// TODO: export class UserManager extends Manager<User, string, RoomMember | ThreadMember> {}
export class UserManager {
	// TODO: methods to get from cache

	// force = bypass cache
	async fetch(id: string, force = false): Promise<User> {
		throw "todo";
	}
}
