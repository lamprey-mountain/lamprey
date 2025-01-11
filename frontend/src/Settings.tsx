import { createResource, createSignal, For, Match, onCleanup, Show, Switch, VoidProps } from "solid-js";
import { useCtx } from "./context.ts";
import { InviteT, MemberT, Pagination, RoleT, RoomT } from "./types.ts";

export const RoomSettings = (props: { room: RoomT }) => {
  const [selectedTab, setSelectedTab] = createSignal("info");
  
  return (
		<div class="settings">
			<header>
				room settings: {selectedTab()}
			</header>
			<nav>
				<ul>
					<For each={["info", "invites", "roles", "members"]}>{tab =>
						<li>
							<button
								onClick={() => setSelectedTab(tab)}
								classList={{ "selected": selectedTab() === tab }}
							>{tab}</button>
						</li>
					}</For>
				</ul>
			</nav>
			<main>
				<Switch>
					<Match when={selectedTab() === "info"}>
						<Info room={props.room} />
					</Match>
					<Match when={selectedTab() === "invites"}>
						<Invites room={props.room} />
					</Match>
					<Match when={selectedTab() === "roles"}>
						<Roles room={props.room} />
					</Match>
					<Match when={selectedTab() === "members"}>
						<Members room={props.room} />
					</Match>
				</Switch>
			</main>
		</div>
  )
}

function Info(props: VoidProps<{ room: RoomT }>) {
  const ctx = useCtx();
  
  const setName = async () => {
		ctx.client.http("PATCH", `/api/v1/rooms/${props.room.id}`, {
			name: await ctx.dispatch({ do: "modal.prompt", text: "name?" })
		})
  }
  
  const setDescription = async () => {
		ctx.client.http("PATCH", `/api/v1/rooms/${props.room.id}`, {
			description: await ctx.dispatch({ do: "modal.prompt", text: "description?" }),
		})
  }
	return (
		<>
			<h2>info</h2>
			<div>room name: {props.room.name}</div>
			<div>room description: {props.room.description}</div>
			<div>room id: <code class="select-all">{props.room.id}</code></div>
		  <button onClick={setName}>set name</button><br />
		  <button onClick={setDescription}>set description</button><br />
		</>
	)
}

function Roles(props: VoidProps<{ room: RoomT }>) {
  const ctx = useCtx();
  
  const [roles, { refetch: fetchRoles }] = createResource<Pagination<RoleT> & { room_id: string }, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.room_id !== room_id) value = undefined;
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1)?.id ?? "00000000-0000-0000-0000-000000000000";
		const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/roles?dir=f&from=${lastId}&limit=100`);
  	return {
  		...batch,
  		items: [...value?.items ?? [], ...batch.items],
  	};
  });
    
  const createRole = async () => {
  	const name = await ctx.dispatch({ do: "modal.prompt", text: "role name?" })
		ctx.client.http("POST", `/api/v1/rooms/${props.room.id}/roles`, {
			name,
		});
  }

  const deleteRole = (role_id: string) => () => {
		ctx.client.http("DELETE", `/api/v1/rooms/${props.room.id}/roles/${role_id}`);
  }
	
	return (
		<>
			<h2>roles</h2>
			<button onClick={fetchRoles}>fetch more</button><br />
			<button onClick={createRole}>create role</button><br />
			<Show when={roles()}>
				<ul>
					<For each={roles()!.items}>{i => (
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
	)
}

function Members(props: VoidProps<{ room: RoomT }>) {
  const ctx = useCtx();
  const [members, { refetch: fetchMembers }] = createResource<Pagination<MemberT> & { room_id: string }, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.room_id !== room_id) value = undefined;
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1)?.user.id ?? "00000000-0000-0000-0000-000000000000";
		const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/members?dir=f&from=${lastId}&limit=100`);
  	return {
  		...batch,
  		items: [...value?.items ?? [], ...batch.items],
  	};
  });

  const addRole = (user_id: string) => async () => {
  	const role_id = await ctx.dispatch({ do: "modal.prompt", text: "role id?" });
		ctx.client.http("PUT", `/api/v1/rooms/${props.room.id}/members/${user_id}/roles/${role_id}`);
  }
  
  const removeRole = (user_id: string) => async () => {
  	const role_id = await ctx.dispatch({ do: "modal.prompt", text: "role id?" })
		ctx.client.http("DELETE", `/api/v1/rooms/${props.room.id}/members/${user_id}/roles/${role_id}`);
  }

  const obs = new IntersectionObserver((ents) => {
  	if (ents.some(i => i.isIntersecting)) fetchMembers();
  });

  onCleanup(() => obs.disconnect());

	return (
		<>
			<h2>members</h2>
			<button onClick={fetchMembers}>fetch more</button>
			<Show when={members()}>
				<ul>
					<For each={members()!.items}>{i => (
						<li>
							<div style="display:flex">
								<div style="margin-right:.25rem">{i.override_name ?? i.user.name}</div>
								<div>
									<For each={i.roles}>
										{i => <button class="spaced" onClick={() => ctx.dispatch({ do: "modal.alert", text: i.id })}>{i.name}</button>}
									</For>
								</div>
								<div style="flex:1"></div>
								<button class="spaced" onClick={addRole(i.user.id)}>add role</button>
								<button class="spaced" onClick={removeRole(i.user.id)}>remove role</button>
							</div>
							<details>
								<summary>json</summary>
								<pre>{JSON.stringify(i, null, 2)}</pre>
							</details>
						</li>
					)}
					</For>
				</ul>
				<div ref={el => obs.observe(el)}></div>
			</Show>
		</>
	)
}

function Invites(props: VoidProps<{ room: RoomT }>) {
  const ctx = useCtx();
  
  const [invites, { refetch: fetchInvites }] = createResource<Pagination<InviteT> & { room_id: string }, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.room_id !== room_id) value = undefined;
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1)?.code ?? "";
		const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/invites?dir=f&from=${lastId}&limit=100`);
  	return {
  		...batch,
  		items: [...value?.items ?? [], ...batch.items],
  	};
  });

  const createInvite = () => {
		ctx.client.http("POST", `/api/v1/rooms/${props.room.id}/invites`, {});
  }
  
  const deleteInvite = (code: string) => {
		ctx.client.http("DELETE", `/api/v1/invites/${code}`);
  }

	return (
		<>
			<h2>invites</h2>
		  <button onClick={createInvite}>create invite</button><br />
			<button onClick={fetchInvites}>fetch more</button><br />
			<Show when={invites()}>
				<ul>
					<For each={invites()!.items}>{i => (
						<li>
							<details>
								<summary>{i.code}</summary>
								<button onClick={() => deleteInvite(i.code)}>delete invite</button>
								<pre>{JSON.stringify(i, null, 2)}</pre>
							</details>
						</li>
					)}
					</For>
				</ul>
			</Show>
		</>
	)
}
