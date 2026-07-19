import { autoUpdate, computePosition, offset, shift } from "@floating-ui/dom";
import { debounce } from "@solid-primitives/scheduled";
import { useNavigate } from "@solidjs/router";
import { createEffect, createSignal, For, onCleanup, Show } from "solid-js";
import { createStore } from "solid-js/store";
import type { PresenceActivity, UserStatus } from "ts-sdk";
import { useApi } from "@/api";
import { Icon } from "@/atoms/Icon";
import { getStatusPath } from "@/avatar/UserAvatar";
import { useCurrentUser } from "@/contexts/currentUser";
import { useMenu } from "@/contexts/menu";
import { useModals } from "@/contexts/modal";
import { useUserPopout } from "@/contexts/user-popout";
import { usePermissions } from "@/hooks/usePermissions";
import { md } from "@/lib/markdown";
import { getThumbFromId } from "@/media/util";
import { icCheck, icCopy, icEdit } from "@/utils/icons";
import { AvatarWithStatus, EditRoles, type UserProps } from "./User";

// TODO: open user profile in room

export function UserProfileEdit(props: UserProps) {
	const api = useApi();
	const { setMenu } = useMenu();
	const nav = useNavigate();
	const { setUserView } = useUserPopout();
	const [, modalctl] = useModals();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;
	const { has: hasPermission } = usePermissions(
		self_id,
		() => props.room_member?.room_id,
		() => undefined,
	);

	function name() {
		let name = null;

		const rm = props.room_member;
		if (rm) name ??= rm.override_name;

		name ??= props.user.name;
		return name;
	}

	const room_member = () => props.room_member;

	const [editRoles, setEditRoles] = createSignal<{ x: number; y: number }>();
	const editRolesClear = () => setEditRoles();
	document.addEventListener("click", editRolesClear);
	onCleanup(() => document.removeEventListener("click", editRolesClear));

	const editProfile = () => {
		nav("/settings");
		setUserView(null);
	};

	// TODO: move copied into a hook
	const [copied, setCopied] = createSignal(false);
	const clearCopied = debounce(() => setCopied(false), 2000);

	const copyUserId = () => {
		navigator.clipboard.writeText(props.user.id);
		setCopied(true);
		clearCopied();
	};

	const [statusMenuVisible, setStatusMenuVisible] = createSignal(false);
	const [statusPersistent, setStatusPersistent] = createSignal(false);
	const [statusMenuRef, setStatusMenuRef] = createSignal<HTMLElement>();
	const [statusMenuPosition, setStatusMenuPosition] = createStore({
		x: 0,
		y: 0,
		strategy: "absolute" as const,
	});
	let statusButtonRef: HTMLButtonElement | undefined;

	createEffect(() => {
		const button = statusButtonRef;
		const menu = statusMenuRef();
		if (!button || !menu || !statusMenuVisible()) return;

		const cleanup = autoUpdate(button, menu, () => {
			computePosition(button, menu, {
				placement: "right-start",
				middleware: [
					offset(8),
					shift({ mainAxis: true, crossAxis: true, padding: 8 }),
				],
			}).then(({ x, y, strategy }) => {
				// TODO: handle this better?
				if (strategy !== "absolute") {
					console.warn("non absolute strategy");
					return;
				}

				setStatusMenuPosition({ x, y, strategy });
			});
		});
		onCleanup(cleanup);
	});

	const debouncedHide = debounce(() => setStatusMenuVisible(false), 100);

	const hideStatusMenu = () => {
		if (statusPersistent()) return;
		debouncedHide();
	};

	const showStatusMenu = () => {
		if (statusPersistent()) return;
		debouncedHide.clear();
		setStatusMenuVisible(true);
	};

	const showStatusMenuPersistent = (e: MouseEvent) => {
		e.stopImmediatePropagation();
		debouncedHide.clear();
		setStatusPersistent(true);
		setStatusMenuVisible(true);
	};

	const closeStatusMenu = () => {
		debouncedHide.clear();
		setStatusPersistent(false);
		setStatusMenuVisible(false);
	};

	document.addEventListener("click", closeStatusMenu);
	onCleanup(() => document.removeEventListener("click", closeStatusMenu));

	const setPresenceText = (text: string) => {
		// inside modalctl.prompt, props.user is undefined
		const user = api.users.get("@self")!;

		const old = user.presence;
		const activities: PresenceActivity[] = [...old.activities];

		const idx = activities.findIndex((i) => i.type === "Custom");
		if (idx !== -1) activities.splice(idx, 1);
		activities.push({ type: "Custom", text });

		api.client.send({
			type: "Presence",
			presence: {
				status: old.status,
				activities,
			},
		});
	};

	const openPresenceTextPrompt = () => {
		setUserView(null);
		modalctl.prompt("status message?", (text) => {
			if (!text) return;
			setPresenceText(text);
		});
	};

	const setPresenceStatus = (status: UserStatus) => {
		api.client.send({
			type: "Presence",
			presence: {
				status,
				activities: [...props.user.presence.activities],
			},
		});
	};

	return (
		<div class="user-profile-edit">
			<div
				class="user-profile"
				onClick={(e) => {
					e.stopPropagation();
					setMenu(null);
				}}
				onKeyDown={(e) => e.key === "Escape" && setMenu(null)}
				tabIndex={0}
				role="button"
			>
				<div
					class="banner"
					style={{
						"background-image":
							(props.user.banner &&
								`url(${getThumbFromId(props.user.banner, 640)})`) ||
							undefined,
					}}
				/>
				<div class="header">
					<AvatarWithStatus user={props.user} animate={true} />
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
					<Show when={props.user.description}>
						<div class="description">
							<h3 class="dim">About Me</h3>
							<div
								class="markdown"
								innerHTML={md(props.user.description ?? "") as string}
							></div>
						</div>
					</Show>

					<Show when={room_member()}>
						<div class="roles">
							<h3 class="dim">Roles</h3>
							<ul>
								<For each={room_member()?.roles}>
									{(role_id) => {
										const role = api.roles.cache.get(role_id);
										return <li>{role?.name ?? "role"}</li>;
									}}
								</For>
								<Show when={hasPermission("RoleApply")}>
									<li>
										<button
											type="button"
											class="edit-roles-btn"
											onClick={(e) => {
												e.stopImmediatePropagation();
												const rect = (
													e.currentTarget as HTMLElement
												).getBoundingClientRect();
												setEditRoles({
													x: rect.x,
													y: rect.y,
												});
											}}
										>
											edit...
										</button>
									</li>
								</Show>
							</ul>
						</div>
					</Show>

					<menu class="menu">
						<button
							class="button"
							ref={statusButtonRef}
							onClick={showStatusMenuPersistent}
							onFocus={showStatusMenu}
							onBlur={hideStatusMenu}
							onMouseEnter={showStatusMenu}
							onMouseLeave={hideStatusMenu}
						>
							<Status status={props.user.presence.status} />
							set status
						</button>
						<button class="button" onClick={editProfile}>
							<Icon src={icEdit} /> edit profile
						</button>
						<button
							class="button"
							classList={{ copied: copied() }}
							onClick={copyUserId}
						>
							<Icon src={copied() ? icCheck : icCopy} color={null} />{" "}
							{copied() ? "copied!" : "copy id"}
						</button>
					</menu>
				</div>

				<Show when={editRoles()}>
					{(ed) => (
						<Show when={room_member()}>
							{(member) => (
								<EditRoles
									x={ed().x}
									y={ed().y}
									user_id={props.user.id}
									room_id={member().room_id}
								/>
							)}
						</Show>
					)}
				</Show>
				<Show when={statusMenuVisible()}>
					<menu
						ref={setStatusMenuRef}
						class="status-menu"
						style={{
							position: statusMenuPosition.strategy,
							left: `${statusMenuPosition.x}px`,
							top: `${statusMenuPosition.y}px`,
							"z-index": 1000,
						}}
						onClick={(e) => e.stopPropagation()}
						onMouseEnter={showStatusMenu}
						onMouseLeave={hideStatusMenu}
					>
						<button class="button" onClick={[setPresenceStatus, "Online"]}>
							<div class="inner">
								<Status status="Online" /> Online
							</div>
						</button>
						<button class="button" onClick={[setPresenceStatus, "Away"]}>
							<div class="inner">
								<Status status="Away" /> Away
							</div>
						</button>
						<button class="button" onClick={[setPresenceStatus, "Busy"]}>
							<div class="inner">
								<Status status="Busy" /> Busy
							</div>
						</button>
						<button class="button" onClick={[setPresenceStatus, "Available"]}>
							<div class="inner">
								<Status status="Available" /> Available
							</div>
						</button>
						<button class="button" onClick={[setPresenceStatus, "Offline"]}>
							<div class="inner">
								<Status status="Offline" /> Offline
							</div>
						</button>
						<hr />
						<button class="button" onClick={openPresenceTextPrompt}>
							<div class="inner">Set status message...</div>
						</button>
					</menu>
				</Show>
			</div>
		</div>
	);
}

const Status = (props: { status: UserStatus }) => {
	return (
		<svg
			aria-hidden="true"
			role="img"
			class="status-indicator small"
			data-status={props.status}
			viewBox="52 52 24 24"
		>
			<path class="indicator" d={getStatusPath(props.status)} />
		</svg>
	);
};
