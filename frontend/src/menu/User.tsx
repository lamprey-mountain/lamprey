import { Menu, Item, Separator } from "./Parts.tsx";

// the context menu for users
export function UserMenu() {
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
			<Item>copy id</Item>
		</Menu>
	);
}
