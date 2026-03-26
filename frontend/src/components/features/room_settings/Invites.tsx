import { For, Show, type VoidProps } from "solid-js";
import { useApi2, useInvites2, useRoomMembers2, useUsers2 } from "@/api";
import type { RoomT } from "../../../types.ts";
import { Avatar } from "../../../User.tsx";
import { Time } from "../../../atoms/Time.tsx";
import { Copyable } from "../../../utils/general";

export function Invites(props: VoidProps<{ room: RoomT }>) {
	const api2 = useApi2();
	const invites2 = useInvites2();
	const users2 = useUsers2();
	const roomMembers2 = useRoomMembers2();

	const invites = invites2.useRoomList(() => props.room.id);

	const createInvite = () => {
		api2.client.http.POST("/api/v1/room/{room_id}/invite", {
			params: {
				path: { room_id: props.room.id },
			},
			body: {},
		});
	};

	const deleteInvite = (code: string) => {
		api2.client.http.DELETE("/api/v1/invite/{invite_code}", {
			params: {
				path: { invite_code: code },
			},
		});
	};

	return (
		<>
			<h2>invites</h2>
			<button class="big primary" onClick={createInvite}>create invite</button>
			<br />
			<br />
			<div class="invites">
				<Show when={invites()} fallback="loading...">
					<header>
						<div class="code">code</div>
						<div class="creator">creator</div>
						<div class="uses">uses</div>
						<div class="expires">expires</div>
					</header>
					<ul>
						<For each={invites()!.state.ids}>
							{(code) => {
								const invite = invites2.cache.get(code);
								if (!invite) return null;
								const i = invite;
								const user = users2.cache.get(i.creator_id);
								const rm = roomMembers2.cache.get(
									`${props.room.id}:${i.creator_id}`,
								);
								const creatorName = () => {
									return rm?.override_name || user?.name || "unknown";
								};
								return (
									<li class="invite">
										<div class="code">
											<Copyable>{i.code}</Copyable>
										</div>
										<div class="creator">
											<Avatar user={i.creator} pad={0} />
											<div class="info">
												<div class="name">{creatorName()}</div>
												<Time date={new Date(i.created_at)} />
											</div>
										</div>
										<div class="uses">
											<span class="mono">{(i as any).uses ?? 0}</span>
											<span class="dim">/</span>
											<span class="mono">
												{(i as any).max_uses ?? "\u221e"}
											</span>
										</div>
										<div class="expires">
											<Show
												when={i.expires_at}
												fallback={<span class="dim">never</span>}
											>
												<Time date={new Date(i.expires_at!)} />
											</Show>
										</div>
										<div>
											<button onClick={() => deleteInvite(i.code)}>
												delete
											</button>
										</div>
									</li>
								);
							}}
						</For>
					</ul>
				</Show>
			</div>
		</>
	);
}
