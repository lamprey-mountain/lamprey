import { debounce } from "@solid-primitives/scheduled";
import { useNavigate } from "@solidjs/router";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { type EditorState, Plugin } from "prosemirror-state";
import type { PreferencesUser, UserWithRelationship } from "sdk";
import {
	createEffect,
	createResource,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { useApi } from "@/api";
import { Icon } from "@/atoms/Icon";
import { Markdown } from "@/atoms/Markdown";
import { createTooltip } from "@/atoms/Tooltip";
import { AvatarWithStatus } from "@/avatar/UserAvatar";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { useMenu } from "@/contexts/menu";
import { getThumbFromId } from "@/media/util";
import type { ChannelT } from "@/types";
import { Copyable } from "@/utils/general";
import {
	icDm,
	icFriendAdd,
	icFriendReject,
	icMemberAdd,
	icMenu,
} from "@/utils/icons";
import { createEditor } from "../features/editor/Editor";
import { schema } from "../features/editor/schema";
import { syntaxHighlightingPlugin } from "../features/search";
import { RoomIcon } from "./User";

// TODO: redesign
// TODO: maybe use <svg> for masking

export function UserPage(props: { user: UserWithRelationship }) {
	const api = useApi();
	const { setMenu } = useMenu();
	const nav = useNavigate();

	const [mutualRooms] = createResource(
		() => props.user.id,
		async (user_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/user/{user_id}/room",
				{
					params: {
						path: { user_id },
						query: {},
					},
				},
			);
			return data;
		},
	);

	const openUserMenu = (e: MouseEvent) => {
		queueMicrotask(() => {
			setMenu({
				type: "user",
				user_id: props.user.id,
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
			const channel = data as ChannelT;
			nav(`/thread/${channel.id}`);
		}
	};

	const preferences = () => props.user.preferences;
	const [note, setNote] = createSignal("");
	createEffect(() => {
		setNote((preferences()?.frontend?.note as string) || "");
	});

	const handleNoteInput = (state: EditorState) => {
		setNote(state.doc.textContent);
		saveNote();
	};

	const saveNote = debounce(() => {
		const localNote = note();
		const currentConfig = preferences() ?? {
			frontend: {},
			voice: { mute: false, volume: 1.0 },
		};
		const { note: remoteNote, ...restFrontend } = currentConfig.frontend ?? {};

		const newConfig: PreferencesUser = {
			...currentConfig,
			frontend: {
				...restFrontend,
				...(localNote ? { note: localNote } : {}),
			},
		};

		api.client.http.PUT("/api/v1/preferences/user/{user_id}", {
			params: { path: { user_id: props.user.id } },
			body: newConfig,
		});
	}, 500);

	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	const friendTooltip = createTooltip({
		tip: () => {
			const rel = props.user.relationship?.relation;
			if (rel === "Friend" || rel === "Outgoing") return "Remove Friend";
			return "Add Friend";
		},
	});
	const dmTooltip = createTooltip({ tip: () => "Send Message" });
	const menuTooltip = createTooltip({ tip: () => "More Options" });

	const noteEditor = createEditor({
		channelId: () => props.user.id,
		autocomplete,
		toolbar,
		initialContent: note,
	});

	return (
		<div class="user-profile-page">
			<div
				class="banner"
				style={{
					"background-image":
						(props.user.banner &&
							`url(${getThumbFromId(props.user.banner)})`) ||
						undefined,
				}}
			/>

			<header class="header">
				<div class="avatar-wrap">
					<AvatarWithStatus user={props.user} animate={true} />
				</div>
				<div class="name-area">
					<div class="name"> {props.user.name} </div>
				</div>
			</header>

			<menu class="actions">
				<Switch>
					<Match when={props.user.relationship?.relation === "Friend"}>
						<button
							type="button"
							class="button icon-button"
							onClick={removeFriend}
							ref={friendTooltip.content}
						>
							<Icon src={icFriendReject} />
						</button>
					</Match>
					<Match when={props.user.relationship?.relation === "Outgoing"}>
						<button
							type="button"
							class="button icon-button"
							onClick={removeFriend}
							ref={friendTooltip.content}
						>
							<Icon src={icFriendReject} />
						</button>
					</Match>
					<Match when={props.user.relationship?.relation === "Incoming"}>
						<button
							type="button"
							class="button icon-button"
							onClick={sendFriendRequest}
							ref={friendTooltip.content}
						>
							<Icon src={icFriendAdd} />
						</button>
					</Match>
					<Match when={!props.user.relationship?.relation}>
						<button
							type="button"
							class="button icon-button"
							onClick={sendFriendRequest}
							ref={friendTooltip.content}
						>
							<Icon src={icFriendAdd} />
						</button>
					</Match>
				</Switch>
				<button
					type="button"
					class="button icon-button"
					onClick={openDm}
					ref={dmTooltip.content}
				>
					<Icon src={icDm} />
				</button>
				<button
					type="button"
					class="button icon-button"
					onClick={openUserMenu}
					ref={menuTooltip.content}
				>
					<Icon src={icMenu} />
				</button>
			</menu>

			<div class="content">
				<h3 class="dim">About Me</h3>
				<div class="description">
					<Show
						when={props.user.description}
						fallback={<div class="dim empty">no bio!</div>}
					>
						{(d) => <Markdown content={d()} />}
					</Show>
				</div>

				<h3 class="dim">Note</h3>
				<div class="note">
					<noteEditor.View
						onChange={handleNoteInput}
						placeholder="Add a note... (only you can see this)"
						submitOnEnter={false}
						channelId={props.user.id}
						autofocus={false}
					/>
				</div>

				<div class="dim">
					id: <Copyable>{props.user.id}</Copyable>
				</div>
			</div>

			<aside class="aside">
				<h3 class="dim">mutual rooms</h3>
				<ul class="mutual-rooms">
					{/* TODO: use actual store/live update */}
					<For each={mutualRooms()?.items ?? []} fallback="no mutual rooms :(">
						{(room) => {
							// TODO: return nicknames in mutual room endpoint
							const member = api.roomMembers.useMember(
								() => room.id,
								() => props.user.id,
							);

							return (
								<li class="mutual-room">
									<a class="mutual-room-link" href={`/room/${room.id}`}>
										<RoomIcon room={room} />
										<div class="info">
											<div>{room.name}</div>
											<Show when={member()?.override_name}>
												{(nick) => (
													<div class="nickname">
														<span class="as">as</span> {nick()}
													</div>
												)}
											</Show>
										</div>
									</a>
								</li>
							);
						}}
					</For>
				</ul>
			</aside>
		</div>
	);
}
