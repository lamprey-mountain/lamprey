import { useApi } from "../api.tsx";
import { Item, Menu, Separator } from "./Parts.tsx";

type RoomMemberMenuProps = {
	room_id: string;
	user_id: string;
};

export function RoomMemberMenu(props: RoomMemberMenuProps) {
	const api = useApi();
	const message = api.room_members.fetch(
		() => props.room_id,
		() => props.user_id,
	);

	const copyUserId = () => navigator.clipboard.writeText(props.user_id);

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(message())));

	return (
		<Menu>
			<Item>block</Item>
			<Item>dm</Item>
			<Separator />
			<Item>kick</Item>
			<Item>ban</Item>
			<Item>mute</Item>
			<Item>roles</Item>
			<Separator />
			<Item onClick={copyUserId}>copy user id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
