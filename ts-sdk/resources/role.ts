import type { api } from "../index";
import type { Uuid } from "../core/uuid";

export type ApiRole = api["Role"];

export type Role = Omit<ApiRole, "id"> & RoleExt;
export type RoleId = Uuid & { readonly __type: "Role" };

export type RoleExt = {
	// async fetch() {}
	// async update() {}
	// async delete() {}
	// toJSON(): ApiRole {}
	// calc permissions in channel?
};

export class RolesManager {
	// room
	// async fetch()
	// async create()
	// async reorder()
}
