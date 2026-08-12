import type { Application, Room } from "sdk";
import { createMemo, For, Show } from "solid-js";
import { useApi, useRooms } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { usePermissions } from "@/hooks/usePermissions.ts";
import { Item, Menu } from "./Parts";

export const InviteApplicationMenu = (props: { app: Application }) => {
	const api = useApi();
	const rooms = useRooms();
	const roomList = rooms.useList();

	const roomItems = createMemo(() => {
		return roomList.ids
			.map((id) => rooms.cache.get(id) ?? null)
			.filter((r): r is Room => r !== null);
	});

	const inviteToRoom = (room_id: string) => {
		api.client.http.POST("/api/v1/app/{app_id}/invite", {
			params: { path: { app_id: props.app.id } },
			body: room_id as any,
		});
	};

	const u = useCurrentUser();
	const self_id = () => u()?.id;

	// TODO: show room icon
	// TODO: show if application/bot is already added to a room

	return (
		<Menu>
			<For each={roomItems() ?? []} fallback="no rooms?">
				{(r) => {
					const perms = usePermissions(
						self_id,
						() => r.id,
						() => undefined,
					);

					return (
						<Show when={perms.has("IntegrationsManage")}>
							<Item onClick={() => inviteToRoom(r.id)}>{r.name}</Item>
						</Show>
					);
				}}
			</For>
		</Menu>
	);
};
