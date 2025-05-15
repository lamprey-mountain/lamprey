import { Show } from "solid-js";
import { useApi } from "./api.tsx";

export const RouteInviteInner = (props: { code: string }) => {
	const api = useApi();
	const invite = api.invites.fetch(() => props.code);

	const name = () => {
		const i = invite();
		if (i?.target.type === "Room") {
			return i.target.room.name;
		}
		return "unknown";
	};

	return (
		<>
			<Show when={invite.loading}>loading...</Show>
			<Show when={invite()}>
				<div class="box">
					invited to {name()} ({invite()?.target.type})
					<br />
					<button>join</button>
				</div>
				<pre>
					{JSON.stringify(invite(), null, 4)}
				</pre>
			</Show>
		</>
	);
};
