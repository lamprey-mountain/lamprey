import { Item, Menu, Separator } from "./Parts.tsx";

type ChannelNavMenuProps = {};

// when right clicking in channel nav but not on a channel (empty space)
export function ChannelNavMenu(_props: ChannelNavMenuProps) {
	return (
		<Menu>
			<Item>create channel</Item>
			<Item>create invite</Item>
		</Menu>
	);
}
