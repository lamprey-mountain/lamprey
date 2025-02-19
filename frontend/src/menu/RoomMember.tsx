import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { Item, Menu, Separator } from "./Parts.tsx";

type RoomMemberMenuProps = {
	room_id: string;
	user_id: string;
};

export function RoomMemberMenu(props: RoomMemberMenuProps) {
	const ctx = useCtx();
	const api = useApi();
	const member = api.room_members.fetch(
		() => props.room_id,
		() => props.user_id,
	);

	const copyUserId = () => navigator.clipboard.writeText(props.user_id);

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(member())));

	const kick = () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "really kick?",
			cont: (conf) => {
				if (!conf) return;
				api.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id: props.room_id,
							user_id: props.user_id,
						},
					},
				});
			},
		});
	};

	return (
		<Menu>
			<Item>block</Item>
			<Item>dm</Item>
			<Separator />
			<Item onClick={kick}>kick</Item>
			<Item>ban</Item>
			<Item>mute</Item>
			<Item>roles</Item>
			<Separator />
			<Item onClick={copyUserId}>copy user id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
