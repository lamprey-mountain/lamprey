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
