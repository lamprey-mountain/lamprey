import { Item, Menu, Separator } from "./Parts.tsx";
import { useCtx } from "../context.ts";
import { useApi2, useRooms2 } from "@/api";
import type { RoomNavItem } from "../RoomNav.tsx";
import { useModals } from "../contexts/modal";

type FolderMenuProps = {
	folder_id: string;
};

export function FolderMenu(props: FolderMenuProps) {
	const ctx = useCtx();
	const api2 = useApi2();
	const [, modalctl] = useModals();

	const getFolder = () => {
		const config = ctx.preferences().frontend.roomNav as RoomNavItem[];
		if (!config) return null;

		return config.find((item: RoomNavItem) =>
			item.type === "folder" && item.id === props.folder_id
		) as (RoomNavItem & { type: "folder" }) | undefined;
	};

	const markAsRead = () => {
		const folder = getFolder();
		if (folder) {
			const api2 = useRooms2();
			for (const item of folder.items) {
				if (item.type === "room") {
					api2.markRead(item.room_id);
				}
			}
		}
	};

	const renameFolder = () => {
		modalctl.prompt("new folder name", (name) => {
			if (!name) return;

			const currentConfig = ctx.preferences().frontend
				.roomNav as RoomNavItem[];
			const newConfig = currentConfig.map((item: RoomNavItem) => {
				if (item.type === "folder" && item.id === props.folder_id) {
					return { ...item, name };
				}
				return item;
			});

			const c = ctx.preferences();
			ctx.setPreferences({
				...c,
				frontend: {
					...c.frontend,
					roomNav: newConfig,
				},
			});
		});
	};

	const logToConsole = () => {
		const folder = getFolder();
		console.log(JSON.parse(JSON.stringify(folder)));
	};

	return (
		<Menu>
			<Item onClick={markAsRead}>mark as read</Item>
			<Item onClick={renameFolder}>rename</Item>
			<Separator />
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
