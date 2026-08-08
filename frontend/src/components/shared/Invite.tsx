import { useNavigate } from "@solidjs/router";
import type { InviteTarget } from "sdk";
import { createEffect, Match, Show, Switch } from "solid-js";
import { useApi } from "@/api";
import { useCtx } from "@/app/context";
import { Markdown } from "@/atoms/Markdown.tsx";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals } from "@/contexts/modal";
import { Avatar, ChannelIconGdm, RoomIcon } from "./User";
import { Status } from "./UserProfileEdit";

const Title = (props: { title?: string }) => {
	createEffect(() => {
		document.title = props.title ?? "";
	});
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

function _isServerTarget(
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
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const invite = api.invites.use(() => props.code);
	const currentUser = useCurrentUser();

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
		const target = invite()?.target;
		if (!target) return;
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

	// TODO: better ui/smoother flow (turn invite component into a form -> name, etc?)
	const [, modalctl] = useModals();
	const joinWithGuest = async () => {
		modalctl.prompt("name?", async (name) => {
			if (!name) return;
			// FIXME: race condition: page reloads after guest is created, possibly before join() is called
			await api.users.createGuest(name);
			await join();
		});
	};

	// TODO: implement login button (use Authenticate?)
	const login = async () => {
		modalctl.alert("todo!");
	};

	const target = () => invite()?.target;
	const room = () => target() && getRoomFromTarget(target());
	const gdm = () => (isGdmTarget(target()!) ? target()!.channel : undefined);
	const description = () =>
		isRoomTarget(target()!)
			? room()?.description
			: isGdmTarget(target()!)
				? gdm()?.description
				: "";
	const memberCount = () =>
		isRoomTarget(target()!)
			? room()?.member_count
			: isGdmTarget(target()!)
				? gdm()?.member_count
				: 0;
	const onlineCount = () =>
		isRoomTarget(target()!)
			? room()?.online_count
			: isGdmTarget(target()!)
				? gdm()?.online_count
				: 0;

	const getMe = useCurrentUser();

	// TODO: fix slash of invite ui before redirect, redirects should be seamless
	createEffect(() => {
		const t = invite()?.target;
		if (!t) return;

		const me = getMe();
		if (!me) return;

		switch (t.type) {
			case "User": {
				// TODO: redirect to user page if already	friends
				break;
			}
			case "Gdm": {
				const isMember = t.channel.recipients?.some((i) => i.id === me.id);
				if (isMember) {
					nav(`/channel/${t.channel.id}`);
				}
				break;
			}
			case "Room": {
				const member = api.roomMembers.cache.get(`${t.room.id}:${me.id}`);
				if (member) {
					nav(
						t.channel?.id ? `/channel/${t.channel.id}` : `/room/${t.room.id}`,
					);
				}
				break;
			}
			case "Server": {
				if (me.registered_at) {
					nav("/");
				}
				break;
			}
		}
	});

	return (
		<>
			<Title title={invite.loading ? "invite" : titleText()} />
			<Show when={invite()} fallback="loading...">
				<div class="invite-wrapper">
					<div>
						<h3 class="dim" style="margin-left:12px;margin-bottom:4px">
							you have been invited to
						</h3>
						<div class="invite">
							<div class="header">
								<InviteAvatar target={invite()?.target!} />

								<div class="info">
									<div style="font-size: 1.3rem;font-weight: bold">
										{name()}
									</div>
									<Show
										when={target()?.type === "Room" || target()?.type === "Gdm"}
									>
										<Markdown content={description() ?? ""} class="markdown" />
										<div class="dim">
											{/*
												TODO: icons for member/online count
												<Status status="Offline" /> {memberCount()} members, <Status status="Online" /> {onlineCount()} online
											*/}
											{memberCount()} members, {onlineCount()} online
										</div>
									</Show>
								</div>
							</div>

							<menu class="menu">
								<Show when={currentUser()}>
									<button type="button" class="button link" onClick={reject}>
										cancel
									</button>
									<button type="button" class="button primary" onClick={join}>
										{joinName()}
									</button>
								</Show>
								<Show when={!currentUser()}>
									<button type="button" class="button link" onClick={login}>
										login
									</button>
									<button
										type="button"
										class="button primary"
										onClick={joinWithGuest}
									>
										join
									</button>
								</Show>
							</menu>

							{/* TODO: show warning if user is missing auth method */}
							<Show when={invite()?.target.type === "Server" && false}>
								<div class="warning">
									<div>you need to add an authentication method first!</div>
									<button type="button" class="button">
										add email
									</button>
									<button type="button" class="button">
										add password
									</button>
									<button type="button" class="button">
										login with oauth
									</button>
								</div>
							</Show>
						</div>
					</div>
				</div>
			</Show>
		</>
	);
};

export const InviteAvatar = (props: { target: InviteTarget }) => {
	// FIXME: make typescript happy
	return (
		<Switch>
			<Match when={isRoomTarget(props.target)}>
				<RoomIcon room={props.target.room} />
			</Match>
			<Match when={isGdmTarget(props.target)}>
				<ChannelIconGdm
					id={props.target.channel.id}
					icon={props.target.channel.icon}
				/>
			</Match>
			<Match when={isUserTarget(props.target)}>
				<Avatar user={props.target.user} />
			</Match>
		</Switch>
	);
};
