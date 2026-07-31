import { Show } from "solid-js";
import { Item, Menu } from "./Parts.tsx";

export type PermissionOverwriteMenuProps = {
	channel_id: string;
	overwrite_id: string;
	overwrite_type: "Role" | "User" | "Everyone";
	onDelete?: () => void;
};

// TODO: use this
// export type PermissionOverwriteMenuProps2 = {
// 	room_id: string;
// 	channel_id: string;
// 	overwrite: PermissionOverwrite;
// };

export function PermissionOverwriteMenu(props: PermissionOverwriteMenuProps) {
	const copyId = () => navigator.clipboard.writeText(props.overwrite_id);

	return (
		<Menu>
			<Show
				when={
					props.overwrite_type === "User" || props.overwrite_type === "Role"
				}
			>
				<Item color="danger" onClick={props.onDelete}>
					{props.overwrite_type === "User"
						? "remove user"
						: props.overwrite_type === "Role"
							? "remove role"
							: "remove permissions"}
				</Item>
				<Item onClick={copyId}>
					{props.overwrite_type === "User"
						? "copy user id"
						: props.overwrite_type === "Role"
							? "copy role id"
							: "copy id"}
				</Item>
			</Show>
			<Show when={props.overwrite_type === "Everyone"}>
				<Item color="danger" onClick={props.onDelete}>
					clear permissions
				</Item>
			</Show>
		</Menu>
	);
}
