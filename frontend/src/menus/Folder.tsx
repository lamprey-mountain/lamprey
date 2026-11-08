import { useRooms } from "@/api";
import { useCtx } from "@/app/context";
import { useModals } from "@/contexts/modal";
import type { RoomNavItem } from "@/types/room-nav";
import { Item, Menu, Separator } from "./Parts.tsx";

type FolderMenuProps = {
	folder_id: string;
};

export function FolderMenu(props: FolderMenuProps) {
	const ctx = useCtx();
	const [, modalctl] = useModals();

	const getFolder = () => {
		const config = ctx.preferences().frontend.roomNav as RoomNavItem[];
		if (!config) return null;

		return config.find(
			(item: RoomNavItem) =>
				item.type === "folder" && item.id === props.folder_id,
		) as (RoomNavItem & { type: "folder" }) | undefined;
	};

	const markAsRead = () => {
		const folder = getFolder();
		if (folder) {
			const api2 = useRooms();
			for (const item of folder.items) {
				if (item.type === "room") {
					api2.markRead(item.room_id);
				}
			}
		}
	};

	const editFolder = () => {
		modalctl.open({ type: "folder_edit", folder_id: props.folder_id });
	};

	const logToConsole = () => {
		const folder = getFolder();
		console.log(JSON.parse(JSON.stringify(folder)));
	};

	return (
		<Menu>
			<Item onClick={markAsRead}>mark as read</Item>
			<Item onClick={editFolder}>edit...</Item>
			<Separator />
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
