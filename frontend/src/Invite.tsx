import { createEffect, Show } from "solid-js";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { useNavigate } from "@solidjs/router";
import { Nav2 } from "./routes.tsx";
import { ChatNav } from "./Nav.tsx";

const Title = (props: { title?: string }) => {
	createEffect(() => document.title = props.title ?? "");
	return undefined;
};

export const RouteInviteInner = (props: { code: string }) => {
	const api = useApi();
	const ctx = useCtx();
	const nav = useNavigate();
	const invite = api.invites.fetch(() => props.code);

	const name = () => {
		const i = invite();
		if (!i) return "unknown";
		switch (i.target.type) {
			case "Room":
				return i.target.room.name;
			case "Thread":
				return i.target.thread.name;
			case "Server":
				return "the server";
			default:
				return "unknown";
		}
	};

	const join = () => {
		ctx.client.http.POST("/api/v1/invite/{invite_code}", {
			params: {
				path: { invite_code: props.code },
			},
		});
	};

	const reject = () => {
		nav("/");
	};

	return (
		<>
			<Title title={invite.loading ? "invite" : `invited to ${name()}`} />
			<Show when={invite()} fallback="loading...">
				<div class="invite" style="padding:8px">
					<div class="box">
						invited to {name()} ({invite()?.target.type})
						<br />
						<button onClick={join}>join</button>
						<button onClick={reject}>reject</button>
					</div>
					<details>
						<summary>json</summary>
						<pre>
							{JSON.stringify(invite(), null, 4)}
						</pre>
					</details>
				</div>
			</Show>
		</>
	);
};
