import {
	autoUpdate,
	computePosition,
	type ReferenceElement,
	shift,
} from "@floating-ui/dom";
import type { Role, RoomMember, ThreadMember, UserWithRelationship } from "sdk";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import { createStore } from "solid-js/store";
import { useApi, useRoomMembers } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { usePermissions } from "@/hooks/usePermissions";

export type UserProps = {
	room_member?: RoomMember;
	thread_member?: ThreadMember;
	user: UserWithRelationship;
};

export const EditRoles = (props: {
	x: number;
	y: number;
	user_id: string;
	room_id: string;
}) => {
	const api = useApi();
	const roomMembers = useRoomMembers();
	const member = roomMembers.use(() => `${props.room_id}:${props.user_id}`);
	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const [menuFloating, setMenuFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute",
	});

	createEffect(() => {
		setMenuParentRef({
			getBoundingClientRect: () => ({
				x: props.x,
				y: props.y,
				left: props.x,
				top: props.y,
				right: props.x,
				bottom: props.y,
				width: 0,
				height: 0,
			}),
		});
	});

	createEffect(() => {
		const reference = menuParentRef();
		const floating = menuRef();
		if (!reference || !floating) return;
		const cleanup = autoUpdate(reference, floating, () => {
			computePosition(reference, floating, {
				middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
				placement: "right-start",
			}).then(({ x, y, strategy }) => {
				setMenuFloating({ x, y, strategy });
			});
		});
		onCleanup(cleanup);
	});

	const handleChecked =
		(r: Role) => (e: InputEvent & { target: HTMLInputElement }) => {
			const role_id = r.id;
			const user_id = member()?.user_id;
			if (!user_id) return;
			if (e.target?.checked) {
				api.client.http.PUT(
					"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
					{
						params: {
							path: {
								room_id: props.room_id,
								role_id,
								user_id,
							},
						},
					},
				);
			} else {
				api.client.http.DELETE(
					"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
					{
						params: {
							path: {
								room_id: props.room_id,
								role_id,
								user_id,
							},
						},
					},
				);
			}
		};

	const getRoles = () =>
		[...api.roles.cache.values()].filter(
			(r) => r.room_id === props.room_id && r.id !== props.room_id,
		);

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;

	const { permissions } = usePermissions(
		self_id,
		() => props.room_id,
		() => undefined,
	);

	return (
		<menu
			class="edit-roles"
			style={{
				translate: `${menuFloating.x}px ${menuFloating.y}px`,
			}}
			ref={setMenuRef}
			onClick={(e) => e.stopImmediatePropagation()}
		>
			<For each={getRoles()}>
				{(r) => (
					<label classList={{ disabled: r.position >= permissions().rank }}>
						<input
							type="checkbox"
							checked={member()?.roles.includes(r.id)}
							onInput={handleChecked(r)}
							disabled={r.position >= permissions().rank}
						/>
						<div>
							<div classList={{ has: member()?.roles.includes(r.id) }}>
								{r.name}
							</div>
							<div class="dim">{r.description}</div>
						</div>
					</label>
				)}
			</For>
		</menu>
	);
};

export { ChannelIcon, ChannelIconGdm } from "@/avatar/ChannelIcon";
export { RoomIcon } from "@/avatar/RoomIcon";
export { Avatar, AvatarWithStatus } from "@/avatar/UserAvatar";
