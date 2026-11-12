import { Show } from "solid-js";
import { useApi, useRoles, useRoomMembers, useUsers } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Time } from "@/atoms/Time";
import {
	formatOrigin,
	hasModerationFlags,
	MemberFlags,
} from "@/components/shared/MemberFlags";
import { Avatar } from "@/components/shared/User";
import { useCurrentUser } from "@/contexts/currentUser";
import { useMenu } from "@/contexts/menu";
import { useModals } from "@/contexts/modal";
import { useNavigate } from "@/contexts/router";
import { usePermissions } from "@/hooks/usePermissions";
import type { RoomT } from "@/types";
import { Copyable, getDate } from "@/utils/general";
import { icCancel } from "@/utils/icons";

export function MemberPanel(props: {
	room: RoomT;
	user_id: string;
	onClose: () => void;
}) {
	const api2 = useApi();
	const roomMembers2 = useRoomMembers();
	const roles2 = useRoles();
	const users2 = useUsers();
	const { setMenu } = useMenu();
	const [, modalCtl] = useModals();
	const nav = useNavigate();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;

	const member = () =>
		roomMembers2.cache.get(`${props.room.id}:${props.user_id}`);
	const user = () => users2.cache.get(props.user_id);
	const name = () => member()?.override_name ?? user()?.name ?? props.user_id;

	const { has: hasPermission, permissions } = usePermissions(
		self_id,
		() => props.room.id,
		() => undefined,
	);

	const { permissions: targetPermissions } = usePermissions(
		() => props.user_id,
		() => props.room.id,
		() => undefined,
	);

	const canModerate = () => permissions().rank > targetPermissions().rank;

	const roleNames = () =>
		(member()?.roles ?? [])
			.map((role_id) => roles2.cache.get(role_id)?.name)
			.filter((name): name is string => !!name);

	const openRolesMenu = (e: MouseEvent) => {
		e.stopPropagation();
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		setMenu({
			type: "member_roles",
			room_id: props.room.id,
			user_id: props.user_id,
			x: rect.left,
			y: rect.bottom,
		});
	};

	const openDm = async () => {
		const { data } = await api2.client.http.POST(
			"/api/v1/user/@self/dm/{target_id}",
			{ params: { path: { target_id: props.user_id } } },
		);
		if (data && "id" in data) nav(`/channel/${data.id}`);
	};

	const toggleMute = () => {
		if (!hasPermission("VoiceMute")) return;
		api2.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
			params: { path: { room_id: props.room.id, user_id: props.user_id } },
			body: { mute: !member()?.mute },
		});
	};

	const toggleDeafen = () => {
		if (!hasPermission("VoiceDeafen")) return;
		api2.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
			params: { path: { room_id: props.room.id, user_id: props.user_id } },
			body: { deaf: !member()?.deaf },
		});
	};

	const kick = () => {
		if (!canModerate()) return;
		modalCtl.open({
			type: "kick",
			room_id: props.room.id,
			user_id: props.user_id,
		});
	};

	const ban = () => {
		if (!canModerate()) return;
		modalCtl.open({
			type: "ban",
			room_id: props.room.id,
			user_id: props.user_id,
		});
	};

	const timeout = () => {
		if (!canModerate()) return;
		modalCtl.open({
			type: "timeout",
			room_id: props.room.id,
			user_id: props.user_id,
		});
	};

	return (
		<Show when={member()}>
			{(m) => (
				<div class="member-panel">
					<header>
						<h3>member</h3>
						<button
							type="button"
							class="button icon-button"
							onClick={props.onClose}
						>
							<Icon src={icCancel} alt="close" />
						</button>
					</header>
					<div class="member-head">
						<Avatar user={user()} pad={6} />
						<div class="names">
							<div class="name">{name()}</div>
							<Show when={name() !== user()?.name}>
								<div class="dim">{user()?.name}</div>
							</Show>
						</div>
					</div>
					<div class="dim">
						id: <Copyable>{props.user_id}</Copyable>
					</div>
					<Show when={hasModerationFlags(m())}>
						<MemberFlags member={m()} />
					</Show>
					<div class="member-info">
						<div class="row">
							<span class="dim">joined</span>
							<span>
								<Time date={getDate(m().joined_at)} />
							</span>
						</div>
						<div class="row">
							<span class="dim">joined via</span>
							<span>{formatOrigin(m().origin)}</span>
						</div>
					</div>
					<div class="actions">
						<button type="button" class="button" onClick={openDm}>
							send message
						</button>
						<Show when={hasPermission("VoiceMute")}>
							<button type="button" class="button" onClick={toggleMute}>
								{m().mute ? "room unmute" : "room mute"}
							</button>
						</Show>
						<Show when={hasPermission("VoiceDeafen")}>
							<button type="button" class="button" onClick={toggleDeafen}>
								{m().deaf ? "room undeafen" : "room deafen"}
							</button>
						</Show>
						<Show when={hasPermission("MemberKick") && canModerate()}>
							<button type="button" class="button danger" onClick={kick}>
								kick
							</button>
						</Show>
						<Show when={hasPermission("MemberBan") && canModerate()}>
							<button type="button" class="button danger" onClick={ban}>
								ban
							</button>
						</Show>
						<Show when={hasPermission("MemberTimeout") && canModerate()}>
							<button type="button" class="button danger" onClick={timeout}>
								timeout
							</button>
						</Show>
					</div>
					<div class="member-roles">
						<h4 class="dim">roles</h4>
						<button
							type="button"
							class="button roles-button"
							onClick={openRolesMenu}
						>
							{roleNames().length > 0 ? roleNames().join(", ") : "no roles"}
						</button>
					</div>
				</div>
			)}
		</Show>
	);
}
