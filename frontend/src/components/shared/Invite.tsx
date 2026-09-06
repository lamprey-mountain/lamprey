import { type RouteSectionProps, useNavigate } from "@solidjs/router";
import type { ApiError, Invite, InviteTarget } from "sdk";
import {
	createEffect,
	createResource,
	ErrorBoundary,
	type JSX,
	Match,
	type ParentProps,
	Show,
	Switch,
} from "solid-js";
import { useApi } from "@/api";
import { useCtx } from "@/app/context";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown.tsx";
import { useCurrentUser } from "@/contexts/currentUser";
import { useModals, useModals2 } from "@/contexts/modal";
import { icWarning } from "@/utils/icons";
import { Avatar, ChannelIconGdm, RoomIcon } from "./User";
import { Status } from "./UserProfileEdit";

const isApiError = (err: unknown): err is ApiError => {
	if (typeof err !== "object") return false;
	if (!err) return false;
	if (!("code" in err)) return false;
	if (!("message" in err)) return false;
	return true;
};

const renderError = (code: string) => {
	switch (code) {
		case "UnknownInvite":
			return "Invite code not found";
		default:
			return code;
	}
};

const InviteError = (props: { error: ApiError }) => (
	<div class="error-container">
		<div class="error-message">
			<Icon src={icWarning} />
			<div>
				<div>
					<span class="error-prefix">Error:</span>{" "}
					{renderError(props.error.code)}
				</div>
				<div class="error-code">{props.error.code}</div>
			</div>
		</div>
	</div>
);

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

type InviteResult =
	| { status: "loading" }
	| { status: "missingCode" }
	| { status: "loaded"; invite: Invite }
	| { status: "apiError"; err: ApiError }
	| { status: "platformError"; err: unknown };

export const RouteInvite = (p: ParentProps<RouteSectionProps>): JSX.Element => {
	const api = useApi();

	const [invite] = createResource<InviteResult, string>(
		() => p.params.code,
		async (code) => {
			if (!code) return { status: "missingCode" };
			try {
				const invite = await api.invites.fetchOrQueue(code);
				return { status: "loaded", invite };
			} catch (err) {
				if (isApiError(err)) {
					return { status: "apiError", err };
				} else {
					return { status: "platformError", err };
				}
			}
		},
	);

	function matches<T extends InviteResult["status"]>(
		ty: T,
	): (InviteResult & { status: T }) | false {
		const i = invite() ?? { status: "loading" };
		if (i.status === ty) {
			return i as InviteResult & { status: T };
		} else {
			return false;
		}
	}

	return (
		<div class="invite-wrapper">
			<Switch>
				<Match when={matches("loading")}>
					{/* TODO: fancier invite loading ui */}
					<div>loading...</div>
				</Match>
				<Match when={matches("missingCode")}>
					<div>invalid invite code</div>
				</Match>
				<Match when={matches("apiError")}>
					{(err) => <InviteError error={err().err} />}
				</Match>
				<Match when={matches("platformError")}>
					<div>internal error</div>
				</Match>
				<Match when={matches("loaded")}>
					{(i) => <RouteInviteInner invite={i().invite} />}
				</Match>
			</Switch>
		</div>
	);
};

const getInviteTargetName = (i: Invite) => {
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

export const RouteInviteInner = (props: { invite: Invite }) => {
	const api = useApi();
	const nav = useNavigate();

	const invite = () => props.invite;

	const name = () => getInviteTargetName(invite());

	const titleText = () => {
		const i = invite();
		const targetType = i.target.type;
		if (targetType === "User") {
			return `${name()} sent a friend request`;
		}
		return `you have been invited to ${name()}`;
	};

	const getMe = useCurrentUser();

	// TODO: fix slash of invite ui before redirect, redirects should be seamless
	createEffect(() => {
		const t = invite().target;

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
		<div>
			<Title title={titleText()} />
			<h3 class="dim" style="margin-left:12px;margin-bottom:4px">
				you have been invited to
			</h3>
			<InviteView invite={invite()} />
		</div>
	);
};

export const InviteView = (props: { invite: Invite }) => {
	const api = useApi();
	const nav = useNavigate();
	const currentUser = useCurrentUser();

	const invite = () => props.invite;

	const name = () => getInviteTargetName(invite());

	const target = () => invite()?.target;
	const room = () => target() && getRoomFromTarget(target());
	const gdm = () => (isGdmTarget(target()!) ? target()!.channel : undefined); // FIXME: typescript error

	// TODO: don't use ternaries
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
		await api.client.http.POST("/api/v1/invite/{invite_code}", {
			params: {
				path: { invite_code: invite().code },
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
	const modals = useModals2();
	const joinWithGuest = async () => {
		const name = await modals.prompt("name?");
		if (!name) return;
		// FIXME: race condition: page reloads after guest is created, possibly before join() is called
		await api.users.createGuest(name);
		await join();
	};

	// TODO: implement login button (use Authenticate?)
	const login = async () => {
		modals.alert("todo!");
	};

	return (
		<div class="invite">
			<div class="header">
				<InviteAvatar target={invite().target!} />

				<div class="info">
					<div style="font-size: 1.3rem;font-weight: bold">{name()}</div>
					<Show when={target()?.type === "Room" || target()?.type === "Gdm"}>
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
					<button type="button" class="button primary" onClick={joinWithGuest}>
						join
					</button>
				</Show>
			</menu>

			{/* TODO: redo server invites, remove the element below */}
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
	);
};

export const InviteAvatar = (props: { target: InviteTarget }) => {
	function matchesType<T extends InviteTarget["type"]>(
		ty: T,
	): (InviteTarget & { type: T }) | false {
		if (props.target.type === ty) {
			return props.target as InviteTarget & { type: T };
		} else {
			return false;
		}
	}

	return (
		<Switch>
			<Match when={matchesType("Room")}>
				{(t) => <RoomIcon room={t().room} />}
			</Match>
			<Match when={matchesType("Gdm")}>
				{(t) => <ChannelIconGdm id={t().channel.id} icon={t().channel.icon} />}
			</Match>
			<Match when={matchesType("User")}>
				{(t) => <Avatar user={t().user} />}
			</Match>
		</Switch>
	);
};
