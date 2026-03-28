import { createEffect, Show } from "solid-js";
import { useApi2, useInvites2 } from "@/api";
import { useCtx } from "./context.ts";
import { md } from "./markdown_utils.tsx";
import { useNavigate } from "@solidjs/router";
import { getThumbFromId } from "./media/util.tsx";
import type { InviteTarget } from "sdk";

const Title = (props: { title?: string }) => {
	createEffect(() => (document.title = props.title ?? ""));
	return undefined;
};

// Type guard functions for InviteTarget
function isRoomTarget(
	target: InviteTarget,
): target is Extract<InviteTarget, { type: "Room" }> {
	return target.type === "Room";
}

function isGdmTarget(
	target: InviteTarget,
): target is Extract<InviteTarget, { type: "Gdm" }> {
	return target.type === "Gdm";
}

function isServerTarget(
	target: InviteTarget,
): target is Extract<InviteTarget, { type: "Server" }> {
	return target.type === "Server";
}

function isUserTarget(
	target: InviteTarget,
): target is Extract<InviteTarget, { type: "User" }> {
	return target.type === "User";
}

function getRoomFromTarget(target: InviteTarget | undefined) {
	if (target && isRoomTarget(target)) return target.room;
	return undefined;
}

export const RouteInviteInner = (props: { code: string }) => {
	const api2 = useApi2();
	const invites2 = useInvites2();
	const ctx = useCtx();
	const nav = useNavigate();
	const invite = invites2.use(() => props.code);

	const name = () => {
		const i = invite();
		if (!i) return "unknown";
		const target = i.target;
		switch (target.type) {
			case "Room":
				return target.room?.name;
			case "Gdm":
				return target.channel?.name;
			case "Server":
				return "a server";
			case "User":
				return target.user?.name;
			default:
				return "unknown";
		}
	};

	const titleText = () => {
		const i = invite();
		if (!i) return "invite";
		const targetType = i.target.type;
		if (targetType === "User") {
			return `${name()} sent a friend request`;
		}
		return `you have been invited to ${name()}`;
	};

	const joinName = () => {
		const i = invite();
		if (!i) return "join";
		switch (i.target.type) {
			case "Room":
				return "join";
			case "Server":
				return "register";
			default:
				return "join";
		}
	};

	const join = async () => {
		await ctx.client.http.POST("/api/v1/invite/{invite_code}", {
			params: {
				path: { invite_code: props.code },
			},
		});
		const target = invite()!.target;
		switch (target.type) {
			case "User":
				if (isUserTarget(target)) return nav(`/user/${target.user.id}`);
				break;
			case "Room":
				if (isRoomTarget(target)) {
					return nav(
						target.channel
							? `/channel/${target.channel.id}`
							: `/room/${target.room.id}`,
					);
				}
				break;
			case "Gdm":
				if (isGdmTarget(target)) return nav(`/channel/${target.channel.id}`);
				break;
			case "Server":
				return nav("/");
		}
	};

	const reject = () => {
		nav("/");
	};

	const target = () => invite()?.target;
	const room = () => target() && getRoomFromTarget(target());
	const roomIcon = () => room()?.icon;
	const roomDescription = () => room()?.description ?? "";
	const roomMemberCount = () => room()?.member_count ?? 0;
	const roomOnlineCount = () => room()?.online_count ?? 0;

	return (
		<>
			<Title title={invite.loading ? "invite" : titleText()} />
			<Show when={invite()} fallback="loading...">
				<div class="invite" style="padding:8px">
					<div>
						<h3 class="dim" style="margin-left:12px;margin-bottom:4px">
							you have been invited to
						</h3>
						<div class="box">
							<div style="display:flex;">
								<Show when={roomIcon()}>
									<img
										src={getThumbFromId(roomIcon()!, 64)}
										class="avatar"
									/>
								</Show>
								<div class="info">
									<div style="font-size: 1.3rem;font-weight: bold">
										{name()}
									</div>
									<Show when={target()?.type === "Room"}>
										<div
											class="markdown"
											innerHTML={md(roomDescription()) as string}
										>
										</div>
										<div class="dim">
											{roomMemberCount()} members, {roomOnlineCount()} online
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
