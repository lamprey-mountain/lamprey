import { createEffect, Show } from "solid-js";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import { useNavigate } from "@solidjs/router";
import { getThumbFromId } from "./media/util.tsx";

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

	const joinName = () => {
		const i = invite();
		if (!i) return "join";
		switch (i.target.type) {
			case "Room":
				return "join";
			case "Thread":
				return "join";
			case "Server":
				return "register";
			default:
				return "join";
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
					<div>
						<h3 class="dim" style="margin-left:12px;margin-bottom:4px">
							you have been invited to
						</h3>
						<div class="box">
							<div style="display:flex;">
								<Show when={invite()?.target.room?.icon}>
									<img
										src={getThumbFromId(invite()?.target.room.icon, 64)}
										class="avatar"
									/>
								</Show>
								<div class="info">
									<div style="font-size: 1.3rem;font-weight: bold">
										{name()}
									</div>
									<Show when={invite()?.target.type === "Room"}>
										<div>{invite()?.target.room.description}</div>
										<div class="dim">
											{invite()?.target.room.member_count} members,{" "}
											{invite()?.target.room.online_count} online
										</div>
									</Show>
									<div style="display:flex;justify-content:end;gap:4px">
										<button class="link" onClick={reject}>cancel</button>
										<button class="primary" onClick={join}>{joinName()}</button>
									</div>
								</div>
							</div>
						</div>
						<Show when={invite()?.target.type === "Server" && false}>
							<div class="warning">
								<div>you need to add an authentication method first!</div>
								<button>add email</button>
								<button>add password</button>
								<button>login with oauth</button>
							</div>
						</Show>
					</div>
				</div>
			</Show>
		</>
	);
};
