import {
	createEffect,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import { Role, RoomMember, RoomMemberOrigin } from "sdk";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { Avatar } from "../User.tsx";
import { Time } from "../Time.tsx";
import { useFloating } from "solid-floating-ui";
import { ReferenceElement, shift } from "@floating-ui/dom";
import { usePermissions } from "../hooks/usePermissions.ts";
import { useModals } from "../contexts/modal";

export function Members(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();
	const api = useApi();
	const members = api.room_members.list(() => props.room.id);

	const editRolesClear = () => setEditRoles();
	document.addEventListener("click", editRolesClear);
	onCleanup(() => document.removeEventListener("click", editRolesClear));

	const removeRole = (user_id: string, role_id: string) => () => {
		const [, modalCtl] = useModals();
		modalCtl.confirm("really remove?", (conf) => {
			if (!conf) return;
			api.client.http.DELETE(
				"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
				{ params: { path: { room_id: props.room.id, role_id, user_id } } },
			);
		});
	};

	const fetchMore = () => {
		api.room_members.list(() => props.room.id);
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(() => bottom() ? [bottom()!] : [], (entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) fetchMore();
		}
	});

	const [editRoles, setEditRoles] = createSignal<
		{ member: RoomMember; x: number; y: number }
	>();

	return (
		<div class="room-settings-members">
			<h2>members</h2>
			<header>
				<div class="name">name</div>
				<div class="joined">joined</div>
			</header>
			<Show when={members()}>
				<ul>
					<For each={members()!.items}>
						{(i) => {
							const user = api.users.fetch(() => i.user_id);
							const name = () =>
								(i.membership === "Join" ? i.override_name : null) ??
									user()?.name;
							return (
								<li>
									<div class="profile">
										<Avatar user={user()} />
										<div>
											<h3 class="name">{name()}</h3>
											<ul class="roles">
												<For each={i.membership === "Join" ? i.roles : []}>
													{(role_id) => {
														const role = api.roles.fetch(
															() => props.room.id,
															() => role_id,
														);
														return (
															<li>
																<button
																	onClick={removeRole(i.user_id, role_id)}
																>
																	{role()?.name ?? "unknown role"}
																</button>
															</li>
														);
													}}
												</For>
												<li class="add">
													<button
														onClick={(e) => {
															e.stopImmediatePropagation();
															setEditRoles({
																member: i,
																x: e.clientX,
																y: e.clientY,
															});
														}}
													>
														<em>add role...</em>
													</button>
												</li>
											</ul>
										</div>
									</div>
									{
										/* TODO
									<div class="notes">
										{i.deaf && <div>deaf</div>}
										{i.mute && <div>mute</div>}
									</div> */
									}
									<div class="joined">
										<Time date={new Date(i.joined_at)} />
										<div class="dim">{formatOrigin(i.origin)}</div>
									</div>
									<div style="flex:1"></div>
									<button
										onClick={(e) => {
											queueMicrotask(() => {
												ctx.setMenu({
													type: "user",
													room_id: props.room.id,
													user_id: i.user_id,
													x: e.clientX,
													y: e.clientY,
													admin: true,
												});
											});
										}}
									>
										options
									</button>
								</li>
							);
						}}
					</For>
				</ul>
				<div ref={setBottom}></div>
			</Show>
			<Show when={editRoles()}>
				<EditRoles
					x={editRoles()!.x}
					y={editRoles()!.y}
					user_id={editRoles()!.member.user_id}
					room={props.room}
				/>
			</Show>
		</div>
	);
}

// TODO: make this an actual context menu?
const EditRoles = (
	props: { x: number; y: number; user_id: string; room: RoomT },
) => {
	const api = useApi();
	const roles = api.roles.list(() => props.room.id);
	const member = api.room_members.fetch(
		() => props.room.id,
		() => props.user_id,
	);
	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();

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

		props.x;
		props.y;
	});

	const menuFloating = useFloating(() => menuParentRef(), () => menuRef(), {
		middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
		placement: "right-start",
	});

	const handleChecked =
		(r: Role) => (e: InputEvent & { target: HTMLInputElement }) => {
			const role_id = r.id;
			const user_id = member()!.user_id;
			if (e.target!.checked) {
				api.client.http.PUT(
					"/api/v1/room/{room_id}/role/{role_id}/member/{user_id}",
					{
						params: {
							path: {
								room_id: props.room.id,
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
								room_id: props.room.id,
								role_id,
								user_id,
							},
						},
					},
				);
			}
		};

	const getRoles = () =>
		(roles()?.items ?? []).filter((r) => r.id !== r.room_id);

	const self_id = () => api.users.cache.get("@self")!.id;

	const { permissions } = usePermissions(
		self_id,
		() => props.room.id,
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
					<label
						classList={{ disabled: r.position >= permissions().rank }}
					>
						<input
							type="checkbox"
							checked={member()!.roles.includes(r.id)}
							onInput={handleChecked(r)}
							disabled={r.position >= permissions().rank}
						/>
						<div>
							<div classList={{ has: member()!.roles.includes(r.id) }}>
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

function formatOrigin(o: RoomMemberOrigin | undefined | null) {
	switch (o?.type) {
		case "Invite":
			return o.code;
		case "BotInstall":
			return "bot install";
		case "Bridged":
			return "bridged";
		case "Creator":
			return "room creator";
		case null:
		case undefined:
			return "unknown";
	}
}
