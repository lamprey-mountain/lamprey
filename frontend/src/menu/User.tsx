import { Match, Show, Switch } from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { usePermissions } from "../hooks/usePermissions.ts";
import { Item, Menu, Separator } from "./Parts.tsx";
import { useVoice } from "../voice-provider.tsx";

type UserMenuProps = {
	user_id: string;
	room_id?: string;
	thread_id?: string;
};

// the context menu for users
// TODO: hide separators when a category has no items
export function UserMenu(props: UserMenuProps) {
	const ctx = useCtx();
	const api = useApi();
	const user = api.users.fetch(() => props.user_id);
	const room_member = props.room_id
		? api.room_members.fetch(() => props.room_id!, () => props.user_id)
		: () => null;
	const self_id = () => api.users.cache.get("@self")!.id;

	const { has: hasPermission, permissions } = usePermissions(
		self_id,
		() => props.room_id,
		() => props.thread_id,
	);

	const { permissions: targetPerms } = usePermissions(
		() => props.user_id,
		() => props.room_id,
		() => props.thread_id,
	);
	const canModerate = () => permissions().rank > targetPerms().rank;

	const userVoiceStates = () =>
		[...api.voiceStates.values()].filter((s) => s.user_id === props.user_id);
	const connectedToVoice = () => userVoiceStates().length;
	const [voice, voiceActions] = useVoice();

	const sendFriendRequest = () => {
		api.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id: props.user_id } },
		});
	};

	const removeFriend = () => {
		api.client.http.DELETE("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id: props.user_id } },
		});
	};

	const blockUser = () => {
		api.client.http.PUT("/api/v1/user/@self/block/{target_id}", {
			params: { path: { target_id: props.user_id } },
		});
	};

	const unblockUser = () => {
		api.client.http.DELETE("/api/v1/user/@self/block/{target_id}", {
			params: { path: { target_id: props.user_id } },
		});
	};

	const copyUserId = () => navigator.clipboard.writeText(props.user_id);

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(user())));

	const kickRoom = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "kick reason",
			cont: (reason) => {
				if (!reason) return;
				api.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id: props.room_id!,
							user_id: props.user_id,
						},
					},
					headers: {
						"X-Reason": reason,
					},
				});
			},
		});
	};

	const kickThread = () => {
		api.client.http.DELETE(
			"/api/v1/thread/{thread_id}/member/{user_id}",
			{
				params: {
					path: {
						thread_id: props.thread_id!,
						user_id: props.user_id,
					},
				},
			},
		);
	};

	const ban = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "ban reason",
			cont: (reason) => {
				if (!reason) return;
				api.client.http.PUT("/api/v1/room/{room_id}/ban/{user_id}", {
					params: {
						path: {
							room_id: props.room_id!,
							user_id: props.user_id,
						},
					},
					headers: {
						"X-Reason": reason,
					},
					body: {},
				});
			},
		});
	};

	const changeNickname = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "new nickname",
			cont: (nick) => {
				if (nick === null) return;
				api.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id: props.room_id!,
							user_id: props.user_id,
						},
					},
					body: {
						override_name: nick || null,
					},
				});
			},
		});
	};

	const disconnect = () => {
		api.client.http.DELETE("/api/v1/voice/{thread_id}/member/{user_id}", {
			params: {
				path: {
					thread_id: userVoiceStates()[0].thread_id,
					user_id: props.user_id,
				},
			},
		});
	};

	const openDm = () => {
		api.client.http.POST("/api/v1/user/@self/dm/{target_id}", {
			params: { path: { target_id: props.user_id } },
		});
	};

	const mute = () => {
		api.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
			params: { path: { room_id: props.room_id!, user_id: props.user_id } },
			body: { mute: !room_member()?.mute },
		});
	};

	const deafen = () => {
		api.client.http.PATCH("/api/v1/room/{room_id}/member/{user_id}", {
			params: { path: { room_id: props.room_id!, user_id: props.user_id } },
			body: { deaf: !room_member()?.deaf },
		});
	};

	const suspend = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "suspend reason",
			cont: (reason) => {
				api.client.http.POST("/api/v1/user/{user_id}/suspend", {
					params: {
						path: {
							user_id: props.user_id,
						},
					},
					headers: {
						"X-Reason": reason ?? "",
					},
					body: {},
				});
			},
		});
	};

	const unsuspend = () => {
		ctx.dispatch({
			do: "modal.prompt",
			text: "suspend reason",
			cont: (reason) => {
				api.client.http.DELETE("/api/v1/user/{user_id}/suspend", {
					params: {
						path: {
							user_id: props.user_id,
						},
					},
					headers: {
						"X-Reason": reason ?? "",
					},
				});
			},
		});
	};

	return (
		<Menu>
			<Show when={props.thread_id}>
				<Item>mention</Item>
			</Show>
			<Show when={user()?.relationship?.relation !== "Block"}>
				<Item onClick={openDm}>dm</Item>
			</Show>
			<Item
				onClick={() =>
					user()?.relationship?.relation === "Block"
						? unblockUser()
						: blockUser()}
			>
				{user()?.relationship?.relation === "Block" ? "unblock" : "block"}
			</Item>
			<Show when={false}>
				<Item>(un)ignore</Item>
			</Show>
			<Switch>
				<Match when={user()?.relationship?.relation === null}>
					<Item onClick={sendFriendRequest}>add friend</Item>
				</Match>
				<Match when={user()?.relationship?.relation === "Friend"}>
					<Item onClick={removeFriend}>remove friend</Item>
				</Match>
				<Match when={user()?.relationship?.relation === "Incoming"}>
					<Item onClick={sendFriendRequest}>accept friend request</Item>
				</Match>
				<Match when={user()?.relationship?.relation === "Outgoing"}>
					<Item onClick={removeFriend}>cancel friend request</Item>
				</Match>
			</Switch>
			<Separator />
			<Show when={hasPermission("MemberManage")}>
				<Item onClick={changeNickname}>change nickname</Item>
			</Show>
			<Show when={hasPermission("MemberKick") && canModerate()}>
				<Item onClick={kickRoom}>kick</Item>
			</Show>
			<Show when={hasPermission("MemberBan") && canModerate()}>
				<Item onClick={ban}>ban</Item>
			</Show>
			<Show when={false}>
				<Item>timeout</Item>
			</Show>
			<Show when={hasPermission("RoleApply")}>
				<Item>roles</Item>
			</Show>
			<Show when={hasPermission("MemberKick") && props.thread_id}>
				<Item onClick={kickThread}>remove from thread</Item>
			</Show>
			<Separator />
			<Show when={props.user_id !== self_id() && connectedToVoice()}>
				{
					/*
TODO: setting volume greater than 1
<li>
	<label style="display:block;padding:0 8px;padding-top:8px">
		<div class="dim">volume</div>
		<input
			type="range"
			min="0"
			max="150"
			step="0.01"
			list="volume-detents"
			value={voice.userConfig.get(props.user_id)?.volume ?? 100}
			onInput={(e) =>
				voice.userConfig.set(props.user_id, {
					...voice.userConfig.get(props.user_id) ??
						{ mute: false, mute_video: false, volume: 100 },
					volume: parseFloat(e.target.value),
				})}
		/>
	</label>
	// TODO: move this to root
	<datalist id="volume-detents">
		<option value="100" />
	</datalist>
</li>
					*/
				}
				<li>
					<label style="display:block;padding:0 8px;padding-top:8px">
						<div class="dim">volume</div>
						<input
							type="range"
							min="0"
							max="100"
							list="volume-detents"
							value={voice.userConfig.get(props.user_id)?.volume ?? 100}
							onInput={(e) =>
								voice.userConfig.set(props.user_id, {
									...voice.userConfig.get(props.user_id) ??
										{ mute: false, mute_video: false, volume: 100 },
									volume: parseFloat(e.target.value),
								})}
						/>
					</label>
				</li>
				<Item
					onClick={() => {
						const c = voice.userConfig.get(props.user_id) ??
							{ mute: false, mute_video: false, volume: 100 };
						c.mute = !c.mute;
						voice.userConfig.set(props.user_id, { ...c });
					}}
				>
					{voice.userConfig.get(props.user_id)?.mute === true
						? "unmute"
						: "mute"}
				</Item>
			</Show>
			<Show when={props.user_id === self_id() && connectedToVoice()}>
				<Item onClick={voiceActions.toggleMic}>
					{voice.muted ? "unmute" : "mute"}
				</Item>
				<Item onClick={voiceActions.toggleDeafened}>
					{voice.deafened ? "undeafen" : "deafen"}
				</Item>
			</Show>
			<Show when={hasPermission("VoiceMute")}>
				<Item onClick={mute}>
					{room_member()?.mute ? "room unmute" : "room mute"}
				</Item>
			</Show>
			<Show when={hasPermission("VoiceDeafen")}>
				<Item onClick={deafen}>
					{room_member()?.deaf ? "room undeafen" : "room deafen"}
				</Item>
			</Show>
			<Show when={hasPermission("VoiceDisconnect") && connectedToVoice()}>
				<Item onClick={disconnect}>disconnect</Item>
			</Show>
			<Show when={hasPermission("VoiceMove") && connectedToVoice()}>
				<Item>move to</Item>
			</Show>
			<Separator />
			<Item onClick={copyUserId}>copy user id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
