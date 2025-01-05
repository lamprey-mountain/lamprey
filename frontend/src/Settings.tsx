import { Room } from "sdk";
import { createResource, createSignal, For, Match, Show, Switch, useContext } from "solid-js";
import { chatctx } from "./context.ts";
import { RoomT } from "./types.ts";

export const RoomSettings = (props: { room: RoomT }) => {
  const [currentInvite, setCurrentInvite] = createSignal();
  const ctx = useContext(chatctx)!;
  
  const setName = () => {
		ctx.client.http("PATCH", `/api/v1/rooms/${props.room.id}`, {
			name: prompt("name?")
		})
  }
  
  const setDescription = () => {
		ctx.client.http("PATCH", `/api/v1/rooms/${props.room.id}`, {
			description: prompt("description?")
		})
  }

  type Pagination<T> = {
  	count: number,
  	items: Array<T>,
  	has_more: boolean,
  }

  const [members, { refetch: fetchMembers }] = createResource<Pagination<any>, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1).user_id ?? "00000000-0000-0000-0000-000000000000";
		const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/members?dir=f&from=${lastId}&limit=100`);
  	return {
  		...batch,
  		items: [...value?.items ?? [], ...batch.items],
  	};
  });
  
  const [roles, { refetch: fetchRoles }] = createResource<Pagination<any>, string>(() => props.room.id, async (room_id, { value }) => {
  	if (value?.has_more === false) return value;
  	const lastId = value?.items.at(-1).user_id ?? "00000000-0000-0000-0000-000000000000";
		const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/roles?dir=f&from=${lastId}&limit=100`);
  	return {
  		...batch,
  		items: [...value?.items ?? [], ...batch.items],
  	};
  });

  const addRole = (user_id: string) => () => {
  	const role_id = prompt("role id?");
		ctx.client.http("PUT", `/api/v1/rooms/${props.room.id}/members/${user_id}/roles/${role_id}`);
  }
  
  const removeRole = (user_id: string) => () => {
  	const role_id = prompt("role id?");
		ctx.client.http("DELETE", `/api/v1/rooms/${props.room.id}/members/${user_id}/roles/${role_id}`);
  }

  const createInvite = async () => {
		const invite = await ctx.client.http("POST", `/api/v1/rooms/${props.room.id}/invites`, {});
		console.log(invite);
		setCurrentInvite(invite);
  }

  const [selectedTab, setSelectedTab] = createSignal("members");
  
  return (
		<div class="flex-1 bg-bg2 text-fg2 grid grid-rows-[48px_1fr] grid-cols-[144px_1fr]">
			<header class="col-span-2 bg-bg3 border-b-[1px] border-b-bg1">
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
						description: {props.room.description}<br />
					  <button onClick={setName}>set name</button><br />
					  <button onClick={setDescription}>set description</button><br />
					</Match>
					<Match when={selectedTab() === "invites"}>
						<h2 class="text-lg">invites</h2>
					  <button onClick={createInvite}>create invite</button><br />
				    last invite code: <code>{currentInvite()?.code}</code><br />
					</Match>
					<Match when={selectedTab() === "roles"}>
						<h2 class="text-lg">rolef</h2>
						<button onClick={fetchRoles}>fetch more</button>
						<Show when={roles()}>
							<ul>
								<For each={roles()!.items}>{i => (
									<li>
										<details>
											<summary>{i.name}</summary>
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
						<button onClick={fetchMembers}>fetch more</button>
						<Show when={members()}>
							<ul>
								<For each={members()!.items}>{i => (
									<li>
										<details>
											<summary>{i.override_name ?? i.user.name}</summary>
											<button onClick={addRole(i.user.id)}>add role</button>
											<button onClick={removeRole(i.user.id)}>remove role</button>
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
