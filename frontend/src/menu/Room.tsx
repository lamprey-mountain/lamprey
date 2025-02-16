import { useNavigate } from "@solidjs/router";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";

// the context menu for rooms
export function RoomMenu(props: { room_id: string }) {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const room = api.rooms.fetch(() => props.room_id);

	const copyId = () => navigator.clipboard.writeText(props.room_id);

	const copyLink = () => {
		const url = `${ctx.client.opts.baseUrl}/room/${props.room_id}`;
		navigator.clipboard.writeText(url);
	};

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(room())));

	const leave = () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "are you sure you want to leave?",
			cont(confirm) {
				if (!confirm) return;
				ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id: props.room_id,
							user_id: api.users.cache.get("@self")!.id,
						},
					},
				});
			},
		});
	};

	const settings = (to: string) => () =>
		nav(`/room/${props.room_id}/settings${to}`);

	return (
		<Menu>
			<Item>mark as read</Item>
			<Item onClick={copyLink}>copy link</Item>
			<RoomNotificationMenu />
			<Separator />
			<Submenu content={"edit"} onClick={settings("")}>
				<Item onClick={settings("")}>info</Item>
				<Item onClick={settings("/invites")}>invites</Item>
				<Item onClick={settings("/roles")}>roles</Item>
				<Item onClick={settings("/members")}>members</Item>
			</Submenu>
			<Item onClick={leave}>leave</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}

function RoomNotificationMenu() {
	return (
		<>
			<Submenu content={"notifications"}>
				<Item>
					<div>default</div>
					<div class="subtext">Uses your default notification setting.</div>
				</Item>
				<Item>
					<div>everything</div>
					<div class="subtext">You will be notified for all messages.</div>
				</Item>
				<Item>
					<div>new threads</div>
					<div class="subtext">You will be notified for new threads.</div>
				</Item>
				<Item>
					<div>watching</div>
					<div class="subtext">Threads and messages mark this room unread.</div>
				</Item>
				<Item>
					<div>mentions</div>
					<div class="subtext">You will only be notified on @mention</div>
				</Item>
			</Submenu>
			<Submenu content={"mute"}>
				<Item>for 15 minutes</Item>
				<Item>for 3 hours</Item>
				<Item>for 8 hours</Item>
				<Item>for 1 day</Item>
				<Item>for 1 week</Item>
				<Item>forever</Item>
			</Submenu>
		</>
	);
}
