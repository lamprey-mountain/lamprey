import { Match, Show, Switch } from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { usePermissions } from "../hooks/usePermissions.ts";
import { Item, Menu, Separator } from "./Parts.tsx";

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
			<Show when={hasPermission("MemberKick")}>
				<Item onClick={kickThread}>remove from thread</Item>
			</Show>
			<Separator />
			<Show when={props.user_id !== self_id() && connectedToVoice()}>
				<Item>volume</Item>
				<Item>mute (for yourself)</Item>
			</Show>
			<Show when={props.user_id === self_id() && connectedToVoice()}>
				<Item>mute</Item>
				<Item>deafen</Item>
			</Show>
			<Show when={hasPermission("VoiceMute")}>
				<Item>room mute</Item>
			</Show>
			<Show when={hasPermission("VoiceDeafen")}>
				<Item>room deafen</Item>
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
