import { RoomMember, ThreadMember, User } from "sdk";
import { useApi } from "./api";
import { For, Show, VoidProps } from "solid-js";
import { Copyable } from "./util";
import { getThumb, getUrl } from "./media/util";

type UserProps = {
	room_member?: RoomMember;
	thread_member?: ThreadMember;
	user: User;
};

export function UserView(props: UserProps) {
	const api = useApi();

	function name() {
		let name = null;
		const tm = props.thread_member;
		if (tm?.membership === "Join") name ??= tm.override_name;

		const rm = props.room_member;
		if (rm?.membership === "Join") name ??= rm.override_name;

		name ??= props.user.name;
		return name;
	}

	function getThumb(media_id: string) {
		const media = api.media.fetchInfo(() => media_id);
		const m = media();
		if (!m) return;
		const tracks = [m.source, ...m.tracks];
		const source =
			tracks.find((s) => s.type === "Thumbnail" && s.height === 64) ??
				tracks.find((s) => s.type === "Image");
		if (source) {
			return getUrl(source);
		} else {
			console.error("no valid avatar source?", m);
		}
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

export const AvatarWithStatus = (props: VoidProps<{ user?: User }>) => {
	const api = useApi();

	function fetchThumb(media_id: string) {
		const media = api.media.fetchInfo(() => media_id);
		const m = media();
		if (!m) return;
		return getUrl(getThumb(m, 64, 64));
	}

	const size = 64;
	const pad = 4;
	const totalSize = size + pad * 2;
	const circPos = size;
	const circRad = 8;
	const circPad = 6;
	return (
		<svg
			class="avatar status-indicator"
			data-status={props.user?.status.type ?? "Offline"}
			viewBox={`0 0 ${totalSize} ${totalSize}`}
			role="img"
			style={{ "--pad": `${pad}px` }}
		>
			{/* not sure if i want avatars to be boxes, circles, rounded boxes, ..? */}
			<mask id="box">
				<rect width={totalSize} height={totalSize} fill="white" />
				<circle cx={circPos} cy={circPos} r={circRad + circPad} fill="black" />
			</mask>
			<mask id="rbox">
				<rect rx="12" width={size} height={size} x={pad} y={pad} fill="white" />
				<circle cx={circPos} cy={circPos} r={circRad + circPad} fill="black" />
			</mask>
			<mask id="circle">
				<circle
					cx={totalSize / 2}
					cy={totalSize / 2}
					r={size / 2}
					fill="white"
				/>
				<circle cx={circPos} cy={circPos} r={circRad + circPad} fill="black" />
			</mask>
			<g mask="url(#box)">
				<rect
					width={size}
					height={size}
					x={pad}
					y={pad}
					fill="oklch(var(--color-bg1))"
				/>
				<Show when={props.user?.avatar}>
					<image
						// temp? i need to crop avatars properly on upload
						preserveAspectRatio="xMidYMid slice"
						width={size}
						height={size}
						x={pad}
						y={pad}
						href={fetchThumb(props.user!.avatar!)!}
					/>
				</Show>
			</g>
			<circle class="indicator" cx={circPos} cy={circPos} r={circRad} />
		</svg>
	);
};
