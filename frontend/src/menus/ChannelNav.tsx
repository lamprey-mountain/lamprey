import { Show } from "solid-js";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useModals } from "@/contexts/modal.tsx";
import { usePermissions } from "@/hooks/usePermissions.ts";
import { Item, Menu } from "./Parts.tsx";

export type ChannelNavMenuProps = {
	room_id: string;
};

// when right clicking in channel nav but not on a channel (empty space)
export function ChannelNavMenu(props: ChannelNavMenuProps) {
	const [, modalctl] = useModals();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;
	const { has: hasPermission } = usePermissions(
		self_id,
		() => props.room_id,
		() => undefined,
	);

	return (
		<Menu>
			<Show when={hasPermission("ChannelManage")}>
				<Item
					onClick={() => {
						modalctl.open({
							type: "channel_create",
							room_id: props.room_id,
						});
					}}
				>
					create channel
				</Item>
			</Show>
			<Show when={hasPermission("InviteCreate")}>
				<Item
					onClick={() =>
						modalctl.open({
							type: "invite_create",
							room_id: props.room_id,
						})
					}
				>
					create invite
				</Item>
			</Show>
		</Menu>
	);
}
