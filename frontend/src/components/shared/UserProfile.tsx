import { useNavigate } from "@solidjs/router";
import {
	createEffect,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import type { Channel, PreferencesUser } from "ts-sdk";
import { useApi } from "@/api";
import { useCurrentUser } from "@/contexts/currentUser";
import { useMenu } from "@/contexts/menu";
import { usePermissions } from "@/hooks/usePermissions";
import { md } from "@/lib/markdown";
import { getThumbFromId } from "@/media/util";
import { Copyable } from "@/utils/general";
import { AvatarWithStatus, EditRoles, type UserProps } from "./User";

export function UserProfile(props: UserProps) {
	const api = useApi();
	const { setMenu } = useMenu();
	const nav = useNavigate();

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

	const sendFriendRequest = () => {
		api.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id: props.user.id } },
		});
	};

	const removeFriend = async () => {
		await api.client.http.DELETE("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id: props.user.id } },
		});
	};

	const openDm = async () => {
		const { data } = await api.client.http.POST(
			"/api/v1/user/@self/dm/{target_id}",
			{
				params: { path: { target_id: props.user.id } },
			},
		);
		if (data) {
			const channel = data as Channel;
			nav(`/thread/${channel.id}`);
		}
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

	return (
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
				<div class="dim">
					id: <Copyable>{props.user.id}</Copyable>
				</div>
				<div class="actions">
					<Switch>
						<Match when={props.user.relationship?.relation === "Friend"}>
							<button type="button" class="button" onClick={removeFriend}>
								Remove Friend
							</button>
						</Match>
						<Match when={props.user.relationship?.relation === "Outgoing"}>
							<button type="button" class="button" onClick={removeFriend}>
								Cancel Request
							</button>
						</Match>
						<Match when={props.user.relationship?.relation === "Incoming"}>
							<button type="button" class="button" onClick={sendFriendRequest}>
								Accept Friend
							</button>
						</Match>
						<Match when={!props.user.relationship?.relation}>
							<button type="button" class="button" onClick={sendFriendRequest}>
								Add Friend
							</button>
						</Match>
					</Switch>
					<button type="button" class="button" onClick={openDm}>
						Message
					</button>
					<button type="button" class="button" onClick={openUserMenu}>
						menu
					</button>
				</div>

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

				<div class="note">
					<h3 class="dim">Note</h3>
					<textarea
						placeholder="Click to add a note"
						value={note()}
						onInput={handleNoteInput}
					/>
				</div>
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
	);
}
