import { createResource, createSignal, For, Match, Show, Switch, useContext } from "solid-js";
import { chatctx } from "./context.ts";
import { InviteT, MemberT, RoleT, RoomT } from "./types.ts";

const CLASS_BUTTON = "px-1 bg-bg3 hover:bg-bg4 my-0.5";
const CLASS_BUTTON2 = `${CLASS_BUTTON} mx-1`;

export const RoomSettings = (props: { room: RoomT }) => {
  const [selectedTab, setSelectedTab] = createSignal("info");
  const ctx = useContext(chatctx)!;
  
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

  type Pagination<T> = {
  	count: number,
  	items: Array<T>,
  	has_more: boolean,
  }

  const [members, { refetch: fetchMembers }] = createResource<Pagination<MemberT>, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1)?.user.id ?? "00000000-0000-0000-0000-000000000000";
		const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/members?dir=f&from=${lastId}&limit=100`);
  	return {
  		...batch,
  		items: [...value?.items ?? [], ...batch.items],
  	};
  });
  
  const [roles, { refetch: fetchRoles }] = createResource<Pagination<RoleT>, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1)?.id ?? "00000000-0000-0000-0000-000000000000";
		const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/roles?dir=f&from=${lastId}&limit=100`);
  	return {
  		...batch,
  		items: [...value?.items ?? [], ...batch.items],
  	};
  });
  
  const [invites, { refetch: fetchInvites }] = createResource<Pagination<InviteT>, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1)?.code ?? "";
		const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/invites?dir=f&from=${lastId}&limit=100`);
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
  
  const createRole = async () => {
  	const name = await ctx.dispatch({ do: "modal.prompt", text: "role name?" })
		ctx.client.http("POST", `/api/v1/rooms/${props.room.id}/roles`, {
			name,
		});
  }

  const deleteRole = (role_id: string) => () => {
		ctx.client.http("DELETE", `/api/v1/rooms/${props.room.id}/roles/${role_id}`);
  }

  const createInvite = () => {
		ctx.client.http("POST", `/api/v1/rooms/${props.room.id}/invites`, {});
  }
  
  const deleteInvite = (code: string) => {
		ctx.client.http("DELETE", `/api/v1/invites/${code}`);
  }
  
  return (
		<div class="flex-1 bg-bg2 text-fg2 grid grid-rows-[auto_1fr] grid-cols-[144px_1fr]">
			<header class="col-span-2 bg-bg3 border-b-[1px] border-b-bg1 p-2">
				room settings: {selectedTab()}
			</header>
			<nav class="bg-bg3 p-1">
				<ul>
					<li>
						<button
							onClick={() => setSelectedTab("info")}
							class="px-1 py-0.25 w-full text-left hover:bg-bg4"
							classList={{ "bg-bg2": selectedTab() === "info" }}
						>info</button>
					</li>
					<li>
						<button
							onClick={() => setSelectedTab("invites")}
							class="px-1 py-0.25 w-full text-left hover:bg-bg4"
							classList={{ "bg-bg2": selectedTab() === "invites" }}
						>invites</button>
					</li>
					<li>
						<button
							onClick={() => setSelectedTab("roles")}
							class="px-1 py-0.25 w-full text-left hover:bg-bg4"
							classList={{ "bg-bg2": selectedTab() === "roles" }}
						>roles</button>
					</li>
					<li>
						<button
							onClick={() => setSelectedTab("members")}
							class="px-1 py-0.25 w-full text-left hover:bg-bg4"
							classList={{ "bg-bg2": selectedTab() === "members" }}
						>members</button>
					</li>
				</ul>
			</nav>
			<main class="p-1 overflow-auto">
				<Switch>
					<Match when={selectedTab() === "info"}>
						<h2 class="text-lg">info</h2>
						<div>room name: {props.room.name}</div>
						<div>room description: {props.room.description}</div>
						<div>room id: <code class="user-select-all">{props.room.id}</code></div>
					  <button class={CLASS_BUTTON} onClick={setName}>set name</button><br />
					  <button class={CLASS_BUTTON} onClick={setDescription}>set description</button><br />
					</Match>
					<Match when={selectedTab() === "invites"}>
						<h2 class="text-lg">invites</h2>
					  <button class={CLASS_BUTTON} onClick={createInvite}>create invite</button><br />
						<button class={CLASS_BUTTON} onClick={fetchInvites}>fetch more</button><br />
						<Show when={invites()}>
							<ul>
								<For each={invites()!.items}>{i => (
									<li>
										<details>
											<summary>{i.code}</summary>
											<button class={CLASS_BUTTON} onClick={() => deleteInvite(i.code)}>delete invite</button>
											<pre>{JSON.stringify(i, null, 2)}</pre>
										</details>
									</li>
								)}
								</For>
							</ul>
						</Show>
					</Match>
					<Match when={selectedTab() === "roles"}>
						<h2 class="text-lg">roles</h2>
						<button class={CLASS_BUTTON} onClick={fetchRoles}>fetch more</button><br />
						<button class={CLASS_BUTTON} onClick={createRole}>create role</button><br />
						<Show when={roles()}>
							<ul>
								<For each={roles()!.items}>{i => (
									<li>
										<details>
											<summary>{i.name}</summary>
											<button class={CLASS_BUTTON} onClick={deleteRole(i.id)}>delete role</button>
											<pre>{JSON.stringify(i, null, 2)}</pre>
										</details>
									</li>
								)}
								</For>
							</ul>
						</Show>
					</Match>
					<Match when={selectedTab() === "members"}>
						<h2 class="text-lg">members</h2>
						<button class={CLASS_BUTTON} onClick={fetchMembers}>fetch more</button>
						<Show when={members()}>
							<ul>
								<For each={members()!.items}>{i => (
									<li>
										<div class="flex">
											<div class="mr-1">{i.override_name ?? i.user.name}</div>
											<div>
												<For each={i.roles}>
													{i => <button class="italic" onClick={() => ctx.dispatch({ do: "modal.alert", text: i.id })}>{i.name}</button>}
												</For>
											</div>
											<div class="flex-1"></div>
											<button class={CLASS_BUTTON2} onClick={addRole(i.user.id)}>add role</button>
											<button class={CLASS_BUTTON2} onClick={removeRole(i.user.id)}>remove role</button>
										</div>
										<details>
											<summary>json</summary>
											<pre>{JSON.stringify(i, null, 2)}</pre>
										</details>
									</li>
								)}
								</For>
							</ul>
						</Show>
					</Match>
				</Switch>
			</main>
		</div>
  )
}
