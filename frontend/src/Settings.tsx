import { createResource, For, onCleanup, Show, VoidProps } from "solid-js";
import { useCtx } from "./context.ts";
import { InviteT, MemberT, Pagination, RoleT, RoomT } from "./types.ts";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";

const tabs = [
	{ name: "info", path: "", component: Info },
	// TODO: { name: "invites", path: "invites", component: Invites },
	// TODO: { name: "roles", path: "roles", component: Roles },
	// TODO: { name: "members", path: "members", component: Members },
];

export const RoomSettings = (props: { room: RoomT; page: string }) => {
	const currentTab = () => tabs.find((i) => i.path === (props.page ?? ""))!;

	return (
		<div class="settings">
			<header>
				room settings: {currentTab()?.name}
			</header>
			<nav>
				<ul>
					<For each={tabs}>
						{(tab) => (
							<li>
								<A href={`/room/${props.room.id}/settings/${tab.path}`}>
									{tab.name}
								</A>
							</li>
						)}
					</For>
				</ul>
			</nav>
			<main>
				<Show when={currentTab()} fallback="unknown page">
					<Dynamic
						component={currentTab()?.component}
						room={props.room}
					/>
				</Show>
			</main>
		</div>
	);
};

function Info(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();

	const setName = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "name?",
			cont(name) {
				if (!name) return;
				ctx.client.http.PATCH("/api/v1/room/{room_id}", {
					params: { path: { room_id: props.room.id } },
					body: { name },
				});
			},
		});
	};

	const setDescription = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "description?",
			cont(description) {
				if (typeof description !== "string") return;
				ctx.client.http.PATCH("/api/v1/room/{room_id}", {
					params: { path: { room_id: props.room.id } },
					body: { description },
				});
			},
		});
	};
	return (
		<>
			<h2>info</h2>
			<div>room name: {props.room.name}</div>
			<div>room description: {props.room.description}</div>
			<div>
				room id: <code class="select-all">{props.room.id}</code>
			</div>
			<button onClick={setName}>set name</button>
			<br />
			<button onClick={setDescription}>set description</button>
			<br />
		</>
	);
}

function Roles(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();

	const [roles, { refetch: fetchRoles }] = createResource<
		Pagination<RoleT> & { room_id: string },
		string
	>(() => props.room.id, async (room_id, { value }) => {
		if (value?.room_id !== room_id) value = undefined;
		if (value?.has_more === false) return value;
		const lastId = value?.items.at(-1)?.id ??
			"00000000-0000-0000-0000-000000000000";
		const batch = await ctx.client.http(
			"GET",
			`/api/v1/room/${room_id}/roles?dir=f&from=${lastId}&limit=100`,
		);
		return {
			...batch,
			items: [...value?.items ?? [], ...batch.items],
		};
	});

	const createRole = async () => {
		const name = await ctx.dispatch({ do: "modal.prompt", text: "role name?" });
		ctx.client.http("POST", `/api/v1/room/${props.room.id}/roles`, {
			name,
		});
	};

	const deleteRole = (role_id: string) => () => {
		ctx.client.http("DELETE", `/api/v1/room/${props.room.id}/roles/${role_id}`);
	};

	return (
		<>
			<h2>roles</h2>
			<button onClick={fetchRoles}>fetch more</button>
			<br />
			<button onClick={createRole}>create role</button>
			<br />
			<Show when={roles()}>
				<ul>
					<For each={roles()!.items}>
						{(i) => (
							<li>
								<details>
									<summary>{i.name}</summary>
									<button onClick={deleteRole(i.id)}>delete role</button>
									<pre>{JSON.stringify(i, null, 2)}</pre>
								</details>
							</li>
						)}
					</For>
				</ul>
			</Show>
		</>
	);
}

function Members(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();
	const [members, { refetch: fetchMembers }] = createResource<
		Pagination<MemberT> & { room_id: string },
		string
	>(() => props.room.id, async (room_id, { value }) => {
		if (value?.room_id !== room_id) value = undefined;
		if (value?.has_more === false) return value;
		const lastId = value?.items.at(-1)?.user.id ??
			"00000000-0000-0000-0000-000000000000";
		const batch = await ctx.client.http(
			"GET",
			`/api/v1/room/${room_id}/members?dir=f&from=${lastId}&limit=100`,
		);
		return {
			...batch,
			items: [...value?.items ?? [], ...batch.items],
		};
	});

	const addRole = (user_id: string) => async () => {
		const role_id = await ctx.dispatch({
			do: "modal.prompt",
			text: "role id?",
		});
		ctx.client.http(
			"PUT",
			`/api/v1/room/${props.room.id}/members/${user_id}/roles/${role_id}`,
		);
	};

	const removeRole = (user_id: string) => async () => {
		const role_id = await ctx.dispatch({
			do: "modal.prompt",
			text: "role id?",
		});
		ctx.client.http(
			"DELETE",
			`/api/v1/room/${props.room.id}/members/${user_id}/roles/${role_id}`,
		);
	};

	const obs = new IntersectionObserver((ents) => {
		if (ents.some((i) => i.isIntersecting)) fetchMembers();
	});

	onCleanup(() => obs.disconnect());

	return (
		<>
			<h2>members</h2>
			<button onClick={fetchMembers}>fetch more</button>
			<Show when={members()}>
				<ul>
					<For each={members()!.items}>
						{(i) => (
							<li>
								<div style="display:flex">
									<div style="margin-right:.25rem">
										{i.override_name ?? i.user.name}
									</div>
									<div>
										<For each={i.roles}>
											{(i) => (
												<button
													class="spaced"
													onClick={() =>
														ctx.dispatch({ do: "modal.alert", text: i.id })}
												>
													{i.name}
												</button>
											)}
										</For>
									</div>
									<div style="flex:1"></div>
									<button class="spaced" onClick={addRole(i.user.id)}>
										add role
									</button>
									<button class="spaced" onClick={removeRole(i.user.id)}>
										remove role
									</button>
								</div>
								<details>
									<summary>json</summary>
									<pre>{JSON.stringify(i, null, 2)}</pre>
								</details>
							</li>
						)}
					</For>
				</ul>
				<div ref={(el) => obs.observe(el)}></div>
			</Show>
		</>
	);
}

function Invites(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();

	const [invites, { refetch: fetchInvites }] = createResource<
		Pagination<InviteT> & { room_id: string },
		string
	>(() => props.room.id, async (room_id, { value }) => {
		if (value?.room_id !== room_id) value = undefined;
		if (value?.has_more === false) return value;
		const lastId = value?.items.at(-1)?.code ?? "";
		const batch = await ctx.client.http(
			"GET",
			`/api/v1/room/${room_id}/invites?dir=f&from=${lastId}&limit=100`,
		);
		return {
			...batch,
			items: [...value?.items ?? [], ...batch.items],
		};
	});

	const createInvite = () => {
		ctx.client.http("POST", `/api/v1/room/${props.room.id}/invites`, {});
	};

	const deleteInvite = (code: string) => {
		ctx.client.http("DELETE", `/api/v1/invites/${code}`);
	};

	return (
		<>
			<h2>invites</h2>
			<button onClick={createInvite}>create invite</button>
			<br />
			<button onClick={fetchInvites}>fetch more</button>
			<br />
			<Show when={invites()}>
				<ul>
					<For each={invites()!.items}>
						{(i) => (
							<li>
								<details>
									<summary>{i.code}</summary>
									<button onClick={() => deleteInvite(i.code)}>
										delete invite
									</button>
									<pre>{JSON.stringify(i, null, 2)}</pre>
								</details>
							</li>
						)}
					</For>
				</ul>
			</Show>
		</>
	);
}
