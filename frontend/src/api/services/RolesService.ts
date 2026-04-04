import { ReactiveMap } from "@solid-primitives/map";
import { ReactiveSet } from "@solid-primitives/set";
import type { Role, RolePatch } from "sdk";
import {
	type Accessor,
	batch,
	createEffect,
	createResource,
	type Resource,
} from "solid-js";
import { BaseService } from "../core/Service";

export class RolesService extends BaseService<Role> {
	protected cacheName = "role";

	private rolesByRoom = new ReactiveMap<string, ReactiveSet<string>>();

	getKey(item: Role): string {
		return item.id;
	}

	// Roles generally don't have a global fetch endpoint in this API version
	// They are usually fetched via listing the room's roles.
	// However, if we need to fetch a specific role, we need the room_id.
	async fetch(_id: string): Promise<Role> {
		throw new Error("Cannot fetch role by ID alone. Use fetchByRoom.");
	}

	async fetchByRoom(room_id: string, role_id: string): Promise<Role> {
		const data = await this.retryWithBackoff<Role>(() =>
			this.client.http.GET("/api/v1/room/{room_id}/role/{role_id}", {
				params: { path: { room_id, role_id } },
			}),
		);
		this.upsert(data);
		return data;
	}

	protected override afterUpsert(role: Role) {
		const r = this.rolesByRoom.get(role.room_id) ?? new ReactiveSet();
		r.add(role.id);
		this.rolesByRoom.set(role.room_id, r);
	}

	protected override afterUpsertBulk(roles: Role[]) {
		const byRoom = new Map<string, Set<string>>();
		for (const role of roles) {
			let s = byRoom.get(role.room_id);
			if (!s) {
				s = new Set();
				byRoom.set(role.room_id, s);
			}
			s.add(role.id);
		}

		batch(() => {
			for (const [room_id, role_ids] of byRoom) {
				const r = this.rolesByRoom.get(room_id) ?? new ReactiveSet();
				for (const id of role_ids) {
					r.add(id);
				}
				this.rolesByRoom.set(room_id, r);
			}
		});
	}

	protected override afterDelete(role_id: string, role?: Role) {
		if (role) {
			this.rolesByRoom.get(role.room_id)?.delete(role_id);
		}
	}

	useRole(
		room_id: Accessor<string>,
		role_id: Accessor<string>,
	): Resource<Role | undefined> {
		const [resource, { mutate }] = createResource(
			() => {
				const r = room_id();
				const rid = role_id();
				return r && rid ? { r, rid } : undefined;
			},
			async ({ r, rid }) => {
				const cached = this.cache.get(rid);
				if (cached) return cached;

				// Dedupe logic could go here similar to BaseService but with composite key
				return this.fetchByRoom(r, rid);
			},
		);

		createEffect(() => {
			const rid = role_id();
			if (!rid) return;
			if (this.cache.has(rid)) {
				mutate(this.cache.get(rid));
			}
		});

		return resource;
	}

	async create(room_id: string, body: { name: string }): Promise<Role> {
		const { data, error } = await this.client.http.POST(
			"/api/v1/room/{room_id}/role",
			{
				params: { path: { room_id } },
				body,
			},
		);
		if (error) throw error;
		this.upsert(data);
		return data;
	}

	async update(
		room_id: string,
		role_id: string,
		body: RolePatch,
	): Promise<Role> {
		const { data, error } = await this.client.http.PATCH(
			"/api/v1/room/{room_id}/role/{role_id}",
			{
				params: { path: { room_id, role_id } },
				body,
			},
		);
		if (error) throw error;
		this.upsert(data);
		return data;
	}

	async deleteRole(room_id: string, role_id: string) {
		const { error } = await this.client.http.DELETE(
			"/api/v1/room/{room_id}/role/{role_id}",
			{
				params: { path: { room_id, role_id } },
			},
		);
		if (error) throw error;
		this.delete(role_id);
	}

	// Member management
	async addMember(room_id: string, role_id: string, user_id: string) {
		const { error } = await this.client.http.PUT(
			"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
			{
				params: { path: { room_id, role_id, user_id } },
			},
		);
		if (error) throw error;
	}

	async removeMember(room_id: string, role_id: string, user_id: string) {
		const { error } = await this.client.http.DELETE(
			"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
			{
				params: { path: { room_id, role_id, user_id } },
			},
		);
		if (error) throw error;
	}

	listByRoom(room_id: string): Role[] {
		if (!room_id) return [];
		const ids = this.rolesByRoom.get(room_id);
		if (!ids) return [];
		return [...ids]
			.map((id) => this.cache.get(id))
			.filter((r): r is Role => r != null);
	}
}
