import { createResource, For, Show, type VoidProps } from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import { Copyable } from "../util.tsx";

export function Integrations(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();
	const api = useApi();

	const [integrations] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/room/{room_id}/integration",
			{ params: { path: { room_id: props.room.id } } },
		);
		return data;
	});

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

	const kick = (user_id: string) => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "kick reason",
			cont(reason) {
				if (!reason) return;
				api.client.http.DELETE(
					"/api/v1/room/{room_id}/member/{user_id}",
					{
						params: { path: { room_id: props.room.id, user_id } },
						headers: { "x-reason": reason },
					},
				);
			},
		});
	};

	const ban = (user_id: string) => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "ban reason",
			cont(reason) {
				if (!reason) return;
				api.client.http.PUT(
					"/api/v1/room/{room_id}/ban/{user_id}",
					{
						params: { path: { room_id: props.room.id, user_id } },
						body: {},
						headers: { "x-reason": reason },
					},
				);
			},
		});
	};

	return (
		<>
			<h2>integrations</h2>
			<button onClick={() => api.roles.list(() => props.room.id)}>
				fetch more
			</button>
			<Show when={integrations()}>
				<ul class="room-settings-members">
					<For each={integrations()!.items}>
						{(i) => {
							const name = () =>
								(i.member.membership === "Join"
									? i.member.override_name
									: null) ??
									i.bot.name;
							return (
								<li>
									<h3 class="name">{name()}</h3>
									<ul class="roles">
										<For
											each={i.member.membership === "Join"
												? i.member.roles
												: []}
										>
											{(role_id) => {
												const role = api.roles.fetch(
													() => props.room.id,
													() => role_id,
												);
												return (
													<li>
														<button onClick={removeRole(i.bot.id, role_id)}>
															{role()?.name ?? "role"}
														</button>
													</li>
												);
											}}
										</For>
										<li class="add">
											<button onClick={addRole(i.bot.id)}>
												<em>add role...</em>
											</button>
										</li>
									</ul>
									<div>
										user id: <Copyable>{i.bot.id}</Copyable>
									</div>
									<div>
										<button onClick={() => kick(i.bot.id)}>kick</button>
										<button onClick={() => ban(i.bot.id)}>ban</button>
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
