import { For, Show, VoidProps } from "solid-js";
import { useCtx } from "./context.ts";
import { RoomT } from "./types.ts";
import { A } from "@solidjs/router";
import { Dynamic } from "solid-js/web";
import { useApi } from "./api.tsx";

const tabs = [
	{ name: "info", path: "", component: Info },
	{ name: "invites", path: "invites", component: Invites },
	{ name: "roles", path: "roles", component: Roles },
	{ name: "members", path: "members", component: Members },
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
	const api = useApi();
	const roles = api.roles.list(() => props.room.id);

	const createRole = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "role name?",
			cont(name) {
				if (!name) return;
				api.client.http.POST("/api/v1/room/{room_id}/role", {
					params: { path: { room_id: props.room.id } },
					body: { name },
				});
			},
		});
	};

	const deleteRole = (role_id: string) => () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "are you sure?",
			cont(confirmed) {
				if (!confirmed) return;
				api.client.http.DELETE("/api/v1/room/{room_id}/role/{role_id}", {
					params: { path: { room_id: props.room.id, role_id } },
				});
			},
		});
	};

	return (
		<>
			<h2>roles</h2>
			<button onClick={api.roles.list(() => props.room.id)}>fetch more</button>
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
	const api = useApi();
	const members = api.room_members.list(() => props.room.id);

	const addRole = (user_id: string) => () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "role id?",
			cont(role_id) {
				if (!role_id) return;
				api.client.http.PUT(
					"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
					{ params: { path: { room_id: props.room.id, role_id, user_id } } },
				);
			},
		});
	};

	const removeRole = (user_id: string) => () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "role id?",
			cont(role_id) {
				if (!role_id) return;
				api.client.http.DELETE(
					"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
					{ params: { path: { room_id: props.room.id, role_id, user_id } } },
				);
			},
		});
	};

	return (
		<>
			<h2>members</h2>
			<button onClick={() => api.roles.list(() => props.room.id)}>
				fetch more
			</button>
			<Show when={members()}>
				<ul>
					<For each={members()!.items}>
						{(i) => {
							const user = api.users.fetch(() => i.user_id);
							return (
								<li>
									<div style="display:flex">
										<div style="margin-right:.25rem">
											{i.override_name ?? user()?.name}
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
										<button class="spaced" onClick={addRole(i.user_id)}>
											add role
										</button>
										<button class="spaced" onClick={removeRole(i.user_id)}>
											remove role
										</button>
									</div>
									<details>
										<summary>json</summary>
										<pre>{JSON.stringify(i, null, 2)}</pre>
									</details>
								</li>
							);
						}}
					</For>
				</ul>
			</Show>
		</>
	);
}

function Invites(props: VoidProps<{ room: RoomT }>) {
	const api = useApi();

	const invites = api.invites.list(() => props.room.id);

	const createInvite = () => {
		api.client.http.POST("/api/v1/room/{room_id}/invite", {
			params: {
				path: { room_id: props.room.id },
			},
		});
	};

	const deleteInvite = (code: string) => {
		api.client.http.DELETE("/api/v1/invite/{invite_code}", {
			params: {
				path: { invite_code: code },
			},
		});
	};

	return (
		<>
			<h2>invites</h2>
			<button onClick={createInvite}>create invite</button>
			<br />
			<button onClick={() => api.invites.list(() => props.room.id)}>
				fetch more
			</button>
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
