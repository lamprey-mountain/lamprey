import type { RoomMember, ThreadMember, User } from "sdk";
import { useApi } from "./api";
import { For, Show, type VoidProps } from "solid-js";
import { Copyable } from "./util";
import { getThumbFromId } from "./media/util";

type UserProps = {
	room_member?: RoomMember;
	thread_member?: ThreadMember;
	user: User;
};

export function UserView(props: UserProps) {
	const api = useApi();

	function name() {
		let name = null;

		const rm = props.room_member;
		if (rm?.membership === "Join") name ??= rm.override_name;

		name ??= props.user.name;
		return name;
	}

	return (
		<div class="user">
			<div class="info">
				<div class="name">
					{name()}
					<Show when={name() !== props.user.name}>
						<span class="dim">({props.user.name})</span>
					</Show>
				</div>
				<div>
					status: {props.user.status.type}
				</div>
				<div>
					id: <Copyable>{props.user.id}</Copyable>
				</div>
				<Show when={props.room_member?.membership === "Join"}>
					<h3>roles</h3>
					<ul>
						<For
							each={(props.room_member as
								| undefined
								| RoomMember & { membership: "Join" })?.roles}
						>
							{(role_id) => {
								const role = api.roles.fetch(
									() => props.room_member!.room_id,
									() => role_id,
								);
								return <li>{role()?.name ?? "role"}</li>;
							}}
						</For>
					</ul>
				</Show>
			</div>
			<AvatarWithStatus user={props.user} />
		</div>
	);
}

export function getColor(id: string) {
	const last = id.at(-1);
	if (!last) return "#ffffff";
	// if (!last) return "oklch(var(--color-bg1))";
	switch (parseInt(last, 16) % 8) {
		case 0:
			return "oklch(var(--color-red))";
		case 1:
			return "oklch(var(--color-green))";
		case 2:
			return "oklch(var(--color-yellow))";
		case 3:
			return "oklch(var(--color-blue))";
		case 4:
			return "oklch(var(--color-magenta))";
		case 5:
			return "oklch(var(--color-cyan))";
		case 6:
			return "oklch(var(--color-orange))";
		case 7:
			return "oklch(var(--color-teal))";
	}
}

type AvatarProps = {
	user?: User;
	pad?: number;
	// room_member?: RoomMember,
	// thread_member?: ThreadMember,
};

export const AvatarWithStatus = (props: VoidProps<AvatarProps>) => {
	const size = 64;
	const pad = () => props.pad ?? 4;
	const totalSize = () => size + pad() * 2;
	const circPos = size;
	const circRad = 8;
	const circPad = 6;
	return (
		<svg
			class="avatar status-indicator"
			data-status={props.user?.status.type ?? "Offline"}
			viewBox={`0 0 ${totalSize()} ${totalSize()}`}
			role="img"
			style={{ "--pad": `${pad()}px` }}
		>
			{/* not sure if i want avatars to be boxes, circles, rounded boxes, ..? */}
			<mask id="rbox">
				<rect
					rx="6"
					width={size}
					height={size}
					x={pad()}
					y={pad()}
					fill="white"
				/>
				<circle cx={circPos} cy={circPos} r={circRad + circPad} fill="black" />
			</mask>
			<g mask="url(#rbox)">
				<rect
					width={size}
					height={size}
					x={pad()}
					y={pad()}
					fill={props.user?.avatar
						? "oklch(var(--color-bg3))"
						: getColor(props.user?.id ?? "")}
				/>
				<Show when={props.user?.avatar}>
					<image
						// temp? i need to crop avatars properly on upload
						preserveAspectRatio="xMidYMid slice"
						width={size}
						height={size}
						x={pad()}
						y={pad()}
						href={getThumbFromId(props.user!.avatar!)!}
					/>
				</Show>
			</g>
			<circle class="indicator" cx={circPos} cy={circPos} r={circRad} />
		</svg>
	);
};

export const Avatar = (props: VoidProps<AvatarProps>) => {
	const size = 64;
	const pad = () => props.pad ?? 4;
	const totalSize = () => size + pad() * 2;
	return (
		<svg
			class="avatar"
			data-status={props.user?.status.type ?? "Offline"}
			viewBox={`0 0 ${totalSize()} ${totalSize()}`}
			role="img"
			style={{ "--pad": `${pad()}px` }}
		>
			{/* not sure if i want avatars to be boxes, circles, rounded boxes, ..? */}
			<mask id="rbox2">
				<rect
					rx="6"
					width={size}
					height={size}
					x={pad()}
					y={pad()}
					fill="white"
				/>
			</mask>
			<g mask="url(#rbox2)">
				<rect
					width={size}
					height={size}
					x={pad()}
					y={pad()}
					fill={props.user?.avatar
						? "oklch(var(--color-bg3))"
						: getColor(props.user?.id ?? "")}
				/>
				<Show when={props.user?.avatar}>
					<image
						// temp? i need to crop avatars properly on upload
						preserveAspectRatio="xMidYMid slice"
						width={size}
						height={size}
						x={pad()}
						y={pad()}
						href={getThumbFromId(props.user!.avatar!)!}
					/>
				</Show>
			</g>
		</svg>
	);
};
