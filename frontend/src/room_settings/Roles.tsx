import { For, Show, VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { RoomT } from "../types.ts";

export function Roles(props: VoidProps<{ room: RoomT }>) {
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
