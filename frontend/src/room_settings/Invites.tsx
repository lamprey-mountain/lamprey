import { For, Show, VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { RoomT } from "../types.ts";

export function Invites(props: VoidProps<{ room: RoomT }>) {
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
