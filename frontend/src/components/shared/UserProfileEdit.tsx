import { useApi } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { useMenu } from "@/contexts/menu";
import { usePermissions } from "@/hooks/usePermissions";
import { md } from "@/lib/markdown";
import { getThumbFromId } from "@/media/util";
import { Copyable } from "@/utils/general";
import { useNavigate } from "@solidjs/router";
import {
	createSignal,
	createEffect,
	onCleanup,
	Show,
	Switch,
	Match,
	For,
} from "solid-js";
import type { Channel, PreferencesUser } from "ts-sdk";
import { AvatarWithStatus, UserProps, EditRoles } from "./User";
import { Icon } from "@/atoms/Icon";
import { icCheck, icCopy, icEdit } from "@/utils/icons";
import { useUserPopout } from "@/contexts/user-popout";
import { getStatusPath } from "@/avatar/UserAvatar";
import { debounce } from "@solid-primitives/scheduled";

// TODO: open user profile in room

export function UserProfileEdit(props: UserProps) {
	const api = useApi();
	const { setMenu } = useMenu();
	const nav = useNavigate();
	const { setUserView } = useUserPopout();

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

	const openUserMenu = (e: MouseEvent) => {
		queueMicrotask(() => {
			setMenu({
				type: "user",
				user_id: props.user.id,
				room_id: props.room_member?.room_id,
				x: e.clientX,
				y: e.clientY,
				admin: false,
			});
		});
	};

	const preferences = () => props.user.preferences;
	const [note, setNote] = createSignal("");
	createEffect(() => {
		setNote((preferences()?.frontend?.note as string) || "");
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
		const currentConfig = preferences() ?? {
			frontend: {},
			voice: { mute: false, volume: 1.0 },
		};
		const { note, ...restFrontend } = currentConfig.frontend ?? {};

		const newConfig: PreferencesUser = {
			...currentConfig,
			frontend: {
				...restFrontend,
				...(noteToSave ? { note: noteToSave } : {}),
			},
		};

		api.client.http.PUT("/api/v1/preferences/user/{user_id}", {
			params: { path: { user_id: props.user.id } },
			body: newConfig,
		});
	};

	const room_member = () => props.room_member;

	const [editRoles, setEditRoles] = createSignal<{ x: number; y: number }>();
	const editRolesClear = () => setEditRoles();
	document.addEventListener("click", editRolesClear);
	onCleanup(() => document.removeEventListener("click", editRolesClear));

	const setStatus = () => {
		// TODO
	};

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
						{/* TODO: show status selection menu on hover */}
						<button class="button" onClick={() => {}}>
							<svg
								aria-hidden="true"
								role="img"
								class="status-indicator"
								data-user-id={props.user.id}
								data-status={props.user.presence.status}
								viewBox="52 52 24 24"
							>
								<path
									class="indicator"
									d={getStatusPath(props.user.presence.status)}
								/>
							</svg>
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
			</div>
		</div>
	);
}
