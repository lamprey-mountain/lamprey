import { Show } from "solid-js";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";

export const RouteInviteInner = (props: { code: string }) => {
	const api = useApi();
	const ctx = useCtx();
	const invite = api.invites.fetch(() => props.code);

	const name = () => {
		const i = invite();
		if (i?.target.type === "Room") {
			return i.target.room.name;
		}
		return "unknown";
	};

	const join = () => {
		ctx.client.http.POST("/api/v1/invite/{invite_code}", {
			params: {
				path: { invite_code: props.code },
			},
		});
	};

	return (
		<>
			<Show when={invite.loading}>loading...</Show>
			<Show when={invite()}>
				<div class="box">
					invited to {name()} ({invite()?.target.type})
					<br />
					<button onClick={join}>join</button>
				</div>
				<pre>
					{JSON.stringify(invite(), null, 4)}
				</pre>
			</Show>
		</>
	);
};
