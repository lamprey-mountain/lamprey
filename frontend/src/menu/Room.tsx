import { RoomT } from "../types.ts";
import { Menu, Item, Separator, Submenu } from "./Parts.tsx";

// the context menu for rooms
export function RoomMenu(props: { room: RoomT }) {
	const copyId = () => navigator.clipboard.writeText(props.room.id);

	return (
		<Menu>
			<Item>mark as read</Item>
			<Item>copy link</Item>
			<RoomNotificationMenu />
			<Separator />
			<Submenu content={"edit"}>
				<Item>info</Item>
				<Item>invites</Item>
				<Item>roles</Item>
				<Item>members</Item>
			</Submenu>
			<Item>leave</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item>inspect</Item>
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

