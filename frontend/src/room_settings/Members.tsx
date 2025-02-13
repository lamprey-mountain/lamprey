import { For, Show, VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { RoomT } from "../types.ts";

export function Members(props: VoidProps<{ room: RoomT }>) {
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

	const removeRole = (user_id: string, role_id: string) => () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "really remove?",
			cont(conf) {
				if (!conf) return;
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
				<ul class="room-settings-members">
					<For each={members()!.items}>
						{(i) => {
							const user = api.users.fetch(() => i.user_id);
							const name = () => (i.membership === "Join" ? i.override_name : null) ?? user()?.name;
							return (
								<li>
									<h3 class="name">{name()}</h3>
									<ul class="roles">
										<For each={i.membership === "Join" ? i.roles : []}>
											{(role_id) => {
												const role = api.roles.fetch(() => props.room.id, () => role_id);
												return <li><button onClick={removeRole(i.user_id, role_id)}>{role()?.name ?? "role"}</button></li>
											}}
										</For>
										<li class="add">
											<button onClick={addRole(i.user_id)}><em>add role...</em></button>
										</li>
									</ul>
									<div>
										user id: <Copyable>{i.user_id}</Copyable>
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

const Copyable = (props: { children: string }) => {
	const ctx = useCtx();
	const copy = () => {
		navigator.clipboard.writeText(props.children);
		ctx.dispatch({ do: "modal.alert", text: "copied!" })
	}

	return <code onClick={copy}>{props.children}</code>
}
