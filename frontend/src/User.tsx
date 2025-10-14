import type { Role, RoomMember, ThreadMember, User, UserConfigUser } from "sdk";
import { useApi } from "./api";
import {
	createEffect,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { Copyable } from "./util";
import { getThumbFromId } from "./media/util";
import { createStore } from "solid-js/store";
import { useCtx } from "./context.ts";
import {
	autoUpdate,
	computePosition,
	type ReferenceElement,
	shift,
} from "@floating-ui/dom";
import { usePermissions } from "./hooks/usePermissions.ts";

type UserProps = {
	room_member?: RoomMember;
	thread_member?: ThreadMember;
	user: User;
};

const EditRoles = (
	props: { x: number; y: number; user_id: string; room_id: string },
) => {
	const api = useApi();
	const roles = api.roles.list(() => props.room_id);
	const member = api.room_members.fetch(
		() => props.room_id,
		() => props.user_id,
	);
	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const [menuFloating, setMenuFloating] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
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
		const cleanup = autoUpdate(
			reference,
			floating,
			() => {
				computePosition(reference, floating, {
					middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
					placement: "right-start",
				}).then(({ x, y, strategy }) => {
					setMenuFloating({ x, y, strategy });
				});
			},
		);
		onCleanup(cleanup);
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
		(roles()?.items ?? []).filter((r) => r.id !== props.room_id);

	const self_id = () => api.users.cache.get("@self")!.id;

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

export function UserView(props: UserProps) {
	const api = useApi();
	const ctx = useCtx();

	const self_id = () => api.users.cache.get("@self")!.id;
	const { has: hasPermission } = usePermissions(
		self_id,
		() => props.room_member?.room_id,
		() => undefined,
	);

	function name() {
		let name = null;

		const rm = props.room_member;
		if (rm?.membership === "Join") name ??= rm.override_name;

		name ??= props.user.name;
		return name;
	}

	const openUserMenu = (e: MouseEvent) => {
		queueMicrotask(() => {
			ctx.setMenu({
				type: "user",
				user_id: props.user.id,
				room_id: props.room_member?.room_id,
				x: e.clientX,
				y: e.clientY,
			});
		});
	};

	const sendFriendRequest = () => {
		api.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id: props.user.id } },
		});
	};

	const openDm = () => {
		api.client.http.POST("/api/v1/user/@self/dm/{target_id}", {
			params: { path: { target_id: props.user.id } },
		});
	};

	const userConfig = () => props.user.user_config;
	const [note, setNote] = createSignal("");
	createEffect(() => {
		setNote((userConfig()?.frontend?.note as string) || "");
	});

	let timeout: NodeJS.Timeout;
	const handleNoteInput = (e: Event) => {
		const newNote = (e.target as HTMLTextAreaElement).value;
		setNote(newNote);
		clearTimeout(timeout);
		timeout = setTimeout(() => {
			saveNote(newNote);
		}, 500);
	};

	const saveNote = (noteToSave: string) => {
		const currentConfig = userConfig() ?? {
			frontend: {},
			voice: { mute: false, volume: 1.0 },
		};
		const { note, ...restFrontend } = currentConfig.frontend ?? {};

		const newConfig: UserConfigUser = {
			...currentConfig,
			frontend: {
				...restFrontend,
				...(noteToSave ? { note: noteToSave } : {}),
			},
		};

		api.client.http.PUT("/api/v1/config/user/{user_id}", {
			params: { path: { user_id: props.user.id } },
			body: newConfig,
		});
	};

	const room_member = () => props.room_member;

	const [editRoles, setEditRoles] = createSignal<{ x: number; y: number }>();
	const editRolesClear = () => setEditRoles();
	document.addEventListener("click", editRolesClear);
	onCleanup(() => document.removeEventListener("click", editRolesClear));

	return (
		<div
			class="user-profile"
			onClick={(e) => {
				e.stopPropagation();
				ctx.setMenu(null);
			}}
		>
			<div
				class="banner"
				style={{
					"background-image": props.user.banner &&
						`url(${getThumbFromId(props.user.banner!, 640)})`,
				}}
			/>
			<div class="header">
				<AvatarWithStatus user={props.user} />
				<div class="name-area">
					<div class="name">
						{name()}
						<Show when={name() !== props.user.name}>
							<span class="dim">({props.user.name})</span>
						</Show>
					</div>
				</div>
			</div>

			<div class="body">
				<div class="dim">
					id: <Copyable>{props.user.id}</Copyable>
				</div>
				<div class="actions">
					<button onClick={sendFriendRequest}>Add Friend</button>
					<button onClick={openDm}>Message</button>
					<button onClick={openUserMenu}>menu</button>
				</div>

				<Show when={props.user.description}>
					<div class="description">
						<h3>About Me</h3>
						<div class="markdown">
							<p>{props.user.description}</p>
						</div>
					</div>
				</Show>

				<Show when={room_member()?.membership === "Join"}>
					<div class="roles">
						<h3 class="dim">
							Roles
						</h3>
						<ul>
							<For each={room_member()!.roles}>
								{(role_id) => {
									const role = api.roles.fetch(
										() => room_member()!.room_id,
										() => role_id,
									);
									return <li>{role()?.name ?? "role"}</li>;
								}}
							</For>
							<Show when={hasPermission("RoleApply")}>
								<li
									role="button"
									onClick={(e) => {
										e.stopImmediatePropagation();
										setEditRoles({
											x: e.clientX,
											y: e.clientY,
										});
									}}
								>
									edit...
								</li>
							</Show>
						</ul>
					</div>
				</Show>

				<div class="note">
					<h3 class="dim">Note</h3>
					<textarea
						placeholder="Click to add a note"
						value={note()}
						onInput={handleNoteInput}
					/>
				</div>
			</div>
			<Show when={editRoles() && room_member()}>
				<EditRoles
					x={editRoles()!.x}
					y={editRoles()!.y}
					user_id={props.user.id}
					room_id={room_member()!.room_id}
				/>
			</Show>
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

export const ThreadIcon = (
	props: { id: string; icon?: string | null; pad?: number },
) => {
	const pad = () => props.pad ?? 4;
	const size = 64;
	const totalSize = () => size + pad() * 2;
	return (
		<svg
			class="avatar"
			viewBox={`0 0 ${totalSize()} ${totalSize()}`}
			role="img"
			style={{ "--pad": `${pad()}px` }}
		>
			<mask id="thread-icon-mask">
				<rect
					rx="6"
					width={size}
					height={size}
					x={pad()}
					y={pad()}
					fill="white"
				/>
			</mask>
			<g mask="url(#thread-icon-mask)">
				<rect
					width={size}
					height={size}
					x={pad()}
					y={pad()}
					fill={props.icon ? "oklch(var(--color-bg3))" : getColor(props.id)}
				/>
				<Show when={props.icon}>
					<image
						preserveAspectRatio="xMidYMid slice"
						width={size}
						height={size}
						x={pad()}
						y={pad()}
						href={getThumbFromId(props.icon!, 64)!}
					/>
				</Show>
			</g>
		</svg>
	);
};
