import { useApi } from "../api.tsx";
import { Item, Menu, Separator } from "./Parts.tsx";

type UserMenuProps = {
	user_id: string;
};

// the context menu for users
export function UserMenu(props: UserMenuProps) {
	const api = useApi();
	const user = api.users.fetch(() => props.user_id);

	const copyUserId = () => navigator.clipboard.writeText(props.user_id);

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(user())));

	return (
		<Menu>
			<Item>block</Item>
			<Item>dm</Item>
			<Separator />
			<Item>mute</Item>
			<Separator />
			<Item onClick={copyUserId}>copy user id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
