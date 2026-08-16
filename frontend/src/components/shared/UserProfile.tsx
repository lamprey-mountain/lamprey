import { debounce } from "@solid-primitives/scheduled";
import { useNavigate } from "@solidjs/router";
import type { EditorState } from "prosemirror-state";
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
import { EmojiButton } from "@/atoms/EmojiButton.tsx";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown.tsx";
import { createTooltip } from "@/atoms/Tooltip";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useCurrentUser } from "@/contexts/currentUser";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useMenu } from "@/contexts/menu";
import { useUserPopout } from "@/contexts/user-popout";
import { usePermissions } from "@/hooks/usePermissions";
import { getThumbFromId } from "@/media/util";
import { Copyable } from "@/utils/general";
import { icDm, icMemberAdd, icMemberRemove, icMore } from "@/utils/icons";
import { createEditor } from "../features/editor/Editor";
import { AvatarWithStatus, EditRoles, type UserProps } from "./User";

export function UserProfile(props: UserProps) {
	const api = useApi();
	const userPopout = useUserPopout();
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

	const close = () => {
		userPopout.setUserView(null);
	};

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
		const target_id = props.user.id;

		const cached = [...api.channels.cache.values()].find(
			(i) => i.type === "Dm" && i.recipients?.some((j) => j.id === target_id),
		);
		if (cached) return cached;

		const { data } = await api.client.http.POST(
			"/api/v1/user/@self/dm/{target_id}",
			{ params: { path: { target_id } } },
		);
		if (!data) return null;
		const channel = data as Channel;
		api.channels.upsert(channel);
		return channel;
	};

	const onOpenDm = async () => {
		const channel = await openDm();
		if (!channel) return;
		close();
		nav(`/thread/${channel.id}`);
	};

	const preferences = () => props.user.preferences;
	const [note, setNote] = createSignal("");
	createEffect(() => {
		setNote((preferences()?.frontend?.note as string) || "");
	});

	const saveNote = debounce((noteToSave: string) => {
		const currentConfig = preferences() ?? {
			frontend: {},
			voice: { mute: false, volume: 1.0 },
		};
		const { note: _n, ...restFrontend } = currentConfig.frontend ?? {};

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
	}, 500);

	const room_member = () => props.room_member;

	const [editRoles, setEditRoles] = createSignal<{ x: number; y: number }>();
	const editRolesClear = () => setEditRoles();
	document.addEventListener("click", editRolesClear);
	onCleanup(() => document.removeEventListener("click", editRolesClear));

	// TODO: combine friend buttons into one button?
	// TODO: button to reject friend request
	const tipFriendRemove = createTooltip({ tip: () => "Remove Friend" });
	// const tipFriendReject = createTooltip({ tip: () => "Reject Friend Request" });
	const tipFriendCancel = createTooltip({ tip: () => "Cancel Friend Request" });
	const tipFriendAccept = createTooltip({ tip: () => "Accept Friend Request" });
	const tipFriendSend = createTooltip({ tip: () => "Send Friend Request" });
	const tipDm = createTooltip({ tip: () => "Send Message" });
	const tipMenu = createTooltip({ tip: () => "More..." });

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const noteEditor = createEditor({
		channelId: () => props.user.id + "-notes",
		toolbar,
		autocomplete,
		initialContent: note,
	});

	const handleNoteInput = (state: EditorState) => {
		setNote(state.doc.textContent);
		saveNote(state.doc.textContent);
	};

	const dmEditor = createEditor({
		// TODO: use actual dm channel id?
		channelId: () => props.user.id + "-dm",
		toolbar,
		autocomplete,
	});

	const [dmEditorState, setDmEditorState] = createSignal<EditorState>();

	createEffect(() => {
		const state = dmEditorState();
		if (state) {
			dmEditor.setState(state);
			dmEditor.focus();
		}
	});

	const onEmojiPick = (emoji: string, _keepOpen?: boolean) => {
		const editorState = dmEditorState();
		if (editorState) {
			const { from, to } = editorState.selection;
			const tr = editorState.tr.insertText(emoji, from, to);
			const newState = editorState.apply(tr);
			setDmEditorState(newState);
		}
	};

	const onDmSubmit = async (text: string) => {
		const channel = await openDm();

		// TODO: show error message if dm failed
		// TODO: hide dm input if you don't have permission to dm this user
		if (!channel) return false;
		close();
		nav(`/thread/${channel.id}`);
		api.messages.send(channel.id, {
			content: text,
			attachments: [],
		});
		return true;
	};

	const onDmChange = (state: EditorState) => {
		setDmEditorState(state);
	};

	const onDmUpload = () => {
		// TODO(future): sending attachments
	};

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
			>
				<menu class="actions">
					<Switch>
						<Match when={props.user.relationship?.relation === "Friend"}>
							<button
								type="button"
								class="button icon-button"
								onClick={removeFriend}
								ref={tipFriendRemove.content}
							>
								<Icon src={icMemberRemove} />
							</button>
						</Match>
						<Match when={props.user.relationship?.relation === "Outgoing"}>
							<button
								type="button"
								class="button icon-button"
								onClick={removeFriend}
								ref={tipFriendCancel.content}
							>
								<Icon src={icMemberRemove} />
							</button>
						</Match>
						<Match when={props.user.relationship?.relation === "Incoming"}>
							<button
								type="button"
								class="button icon-button"
								onClick={sendFriendRequest}
								ref={tipFriendAccept.content}
							>
								<Icon src={icMemberAdd} />
							</button>
						</Match>
						<Match when={!props.user.relationship?.relation}>
							<button
								type="button"
								class="button icon-button"
								onClick={sendFriendRequest}
								ref={tipFriendSend.content}
							>
								<Icon src={icMemberAdd} />
							</button>
						</Match>
					</Switch>
					<button
						type="button"
						class="button icon-button"
						onClick={onOpenDm}
						ref={tipDm.content}
					>
						<Icon src={icDm} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={openUserMenu}
						ref={tipMenu.content}
					>
						<Icon src={icMore} />
					</button>
				</menu>
			</div>
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

				<Show when={props.user.description}>
					{(desc) => (
						<div class="description">
							<h3 class="dim">About Me</h3>
							<Markdown content={desc()} />
						</div>
					)}
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
					<noteEditor.View
						onChange={handleNoteInput}
						placeholder="Add a note... (only you can see this)"
						submitOnEnter={false}
						channelId={props.user.id + "-notes"}
						autofocus={false}
					/>
				</div>

				<div class="dm-input">
					<dmEditor.View
						onSubmit={onDmSubmit}
						onChange={onDmChange}
						onUpload={onDmUpload}
						channelId={props.user.id}
						placeholder={`Message @${props.user.name}...`}
					/>
					<EmojiButton picked={onEmojiPick} />
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
