import { Show } from "solid-js";
import { Item, Menu } from "./Parts.tsx";

export function PermissionOverwriteMenu(props: {
	channel_id: string;
	overwrite_id: string;
	overwrite_type: "Role" | "User" | "Everyone";
	onDelete?: () => void;
}) {
	const copyId = () => navigator.clipboard.writeText(props.overwrite_id);

	return (
		<Menu>
			<Show
				when={
					props.overwrite_type === "User" || props.overwrite_type === "Role"
				}
			>
				<Item onClick={props.onDelete}>remove permissions</Item>
				<Item onClick={copyId}>
					{props.overwrite_type === "User"
						? "copy user id"
						: props.overwrite_type === "Role"
							? "copy role id"
							: "copy id"}
				</Item>
			</Show>
			<Show when={props.overwrite_type === "Everyone"}>
				<Item onClick={props.onDelete}>clear permissions</Item>
			</Show>
		</Menu>
	);
}
