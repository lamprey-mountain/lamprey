import { Item, Menu, Separator } from "./Parts.tsx";
import { useCtx } from "../context.ts";
import { useApi } from "../api.tsx";
import type { RoomNavItem } from "../RoomNav.tsx";
import { useModals } from "../contexts/modal";

type FolderMenuProps = {
	folder_id: string;
};

export function FolderMenu(props: FolderMenuProps) {
	const ctx = useCtx();
	const api = useApi();

	const getFolder = () => {
		const config = ctx.userConfig().frontend.roomNav as RoomNavItem[];
		if (!config) return null;

		return config.find((item: RoomNavItem) =>
			item.type === "folder" && item.id === props.folder_id
		) as (RoomNavItem & { type: "folder" }) | undefined;
	};

	const markAsRead = () => {
		const folder = getFolder();
		if (folder) {
			for (const item of folder.items) {
				if (item.type === "room") {
					api.rooms.markRead(item.room_id);
				}
			}
		}
	};

	const renameFolder = () => {
		const [, modalCtl] = useModals();
		modalCtl.prompt("new folder name", (name) => {
			if (!name) return;

			const currentConfig = ctx.userConfig().frontend
				.roomNav as RoomNavItem[];
			const newConfig = currentConfig.map((item: RoomNavItem) => {
				if (item.type === "folder" && item.id === props.folder_id) {
					return { ...item, name };
				}
				return item;
			});

			const c = ctx.userConfig();
			ctx.setUserConfig({
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
