import { Show } from "solid-js";
import { useApi } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { useNavigate } from "@/contexts/router";
import { Item, Menu } from "./Parts";

export const SettingsMenu = () => {
	const api = useApi();
	const nav = useNavigate();
	const user = useCurrentUser();

	// TODO: only show server settings if user has a relevant permission?

	return (
		<Menu>
			<Item onClick={() => nav("/settings")}>Profile</Item>
			<Item
				onClick={() =>
					nav("/room/00000000-0000-7000-0000-736572766572/settings")
				}
			>
				Server
			</Item>
			<Show when={user()}>
				<Item onClick={() => api.logout()} color="danger">
					Log out
				</Item>
			</Show>
		</Menu>
	);
};
