import { RoomMember, ThreadMember, User } from "sdk";
import { useApi } from "./api";
import { For, Show, VoidProps } from "solid-js";
import { Copyable } from "./util";
import { getUrl } from "./media/util";

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
				<div style="display:flex;align-items:center;gap:1px">
					status:
					<StatusIndicator user={props.user} />
					{props.user.status.type}
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
			<Show when={props.user.avatar} fallback={<div class="avatar"></div>}>
				<div class="avatar">
					<img src={getThumb(props.user.avatar!)} class="avatar" />
				</div>
			</Show>
		</div>
	);
}

export const StatusIndicator = (props: VoidProps<{ user?: User }>) => {
	return (
		<svg
			class="status-indicator"
			data-status={props.user?.status.type ?? "Offline"}
			viewBox="0 0 10 10"
		>
			<ellipse cx="5" cy="5" rx="4" ry="4" />
		</svg>
	);
};
