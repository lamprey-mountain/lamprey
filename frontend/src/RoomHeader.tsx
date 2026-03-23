import type { Room } from "sdk";
import { useCtx } from "./context.ts";
import { SearchInput } from "./components/features/chat/SearchInput.tsx";
import icMembers from "./assets/members.png";

type RoomHeaderProps = {
	room: Room;
};

export const RoomHeader = (
	props: RoomHeaderProps,
) => {
	const ctx = useCtx();

	const toggleMembers = () => {
		const c = ctx.preferences();
		ctx.setPreferences({
			...c,
			frontend: {
				...c.frontend,
				showMembers: !(c.frontend.showMembers ?? true),
			},
		});
	};

	return (
		<header
			class="chat-header menu-room"
			style="display:flex"
			data-room-id={props.room.id}
		>
			<b>home</b>
			<div style="flex:1"></div>
			<SearchInput room={props.room} />
			<button
				onClick={toggleMembers}
				title="Show members"
			>
				<img class="icon" src={icMembers} />
			</button>
		</header>
	);
};
