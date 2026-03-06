import { useNavigate } from "@solidjs/router";
import { createResource, createSignal, Show } from "solid-js";
import { timeAgo } from "../Time.tsx";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { usePermissions } from "../hooks/usePermissions.ts";
import { useModals } from "../contexts/modal";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";
import { Checkbox } from "../icons.tsx";
import { CheckboxOption } from "../atoms/CheckboxOption";
import { useCurrentUser } from "../contexts/currentUser.tsx";

// the context menu for rooms
export function RoomMenu(props: { room_id: string }) {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const room = api.rooms.fetch(() => props.room_id);
	const [, modalctl] = useModals();

	const currentUser = useCurrentUser();
	const self_id = () => currentUser()?.id;
	const { has: hasPermission } = usePermissions(
		self_id,
		() => props.room_id,
		() => undefined,
	);

	const copyId = () => navigator.clipboard.writeText(props.room_id);

	const copyLink = () => {
		const url = `${ctx.client.opts.apiUrl}/room/${props.room_id}`;
		navigator.clipboard.writeText(url);
	};

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(room())));

	const leave = () => {
		modalctl.confirm("are you sure you want to leave?", (confirm) => {
			if (!confirm) return;
			ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
				params: {
					path: {
						room_id: props.room_id,
						user_id: currentUser()?.id || "",
					},
				},
			});
		});
	};

	const settings = (to: string) => () =>
		nav(`/room/${props.room_id}/settings${to}`);

	return (
		<Menu>
			<Item onClick={() => api.rooms.markRead(props.room_id)}>
				mark as read
			</Item>
			<Item onClick={copyLink}>copy link</Item>
			<Item
				onClick={() =>
					modalctl.open({
						type: "privacy",
						room_id: props.room_id,
					})}
			>
				privacy
			</Item>
			<Show when={room()}>
				{(r) => <RoomNotificationMenu room={r()} />}
			</Show>
			<Separator />
			<Show when={hasPermission("ChannelManage")}>
				<Item
					onClick={() => {
						modalctl.open({
							type: "channel_create",
							room_id: props.room_id,
							cont: (data) => {
								if (!data) return;
								ctx.client.http.POST("/api/v1/room/{room_id}/channel", {
									params: {
										path: { room_id: props.room_id },
									},
									body: {
										name: data.name,
										type: data.type,
									},
								});
							},
						});
					}}
				>
					create channel
				</Item>
			</Show>
			<Submenu content={"edit"} onClick={settings("")}>
				<Item onClick={settings("")}>info</Item>
				<Item onClick={settings("/invites")}>invites</Item>
				<Item onClick={settings("/roles")}>roles</Item>
				<Item onClick={settings("/members")}>members</Item>
				<Item onClick={settings("/integrations")}>integrations</Item>
				<Item onClick={settings("/emoji")}>emoji</Item>
				<Item onClick={settings("/logs")}>audit log</Item>
				<Item onClick={settings("/metrics")}>metrics</Item>
			</Submenu>
			<Item onClick={leave} color="danger">leave</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}

function RoomNotificationMenu(props: { room: import("sdk").Room }) {
	const api = useApi();
	const roomConfig = () => props.room.preferences;

	const setNotifs = (notifs: Partial<import("sdk").NotifsRoom>) => {
		const current = roomConfig() ?? {
			notifs: { mention_everyone: true, mention_roles: true },
			frontend: {},
		};
		const newConfig = {
			...current,
			notifs: { ...current.notifs, ...notifs },
		};
		api.rooms.cache.set(props.room.id, {
			...props.room,
			preferences: newConfig as any,
		});
		api.client.http.PUT("/api/v1/preferences/room/{room_id}", {
			params: { path: { room_id: props.room.id } },
			body: newConfig as any,
		});
	};

	const setMute = (duration_ms: number | null) => {
		const expires_at = duration_ms === null
			? null
			: new Date(Date.now() + duration_ms).toISOString();
		setNotifs({ mute: { expires_at } });
	};

	const unmute = () => setNotifs({ mute: undefined });

	const isMuted = () => {
		const c = roomConfig();
		if (!c?.notifs.mute) return false;
		if (!c.notifs.mute.expires_at) return true;
		return Date.parse(c.notifs.mute.expires_at) > Date.now();
	};

	const fifteen_mins = 15 * 60 * 1000;
	const three_hours = 3 * 60 * 60 * 1000;
	const eight_hours = 8 * 60 * 60 * 1000;
	const one_day = 24 * 60 * 60 * 1000;
	const one_week = 7 * one_day;

	const [, modalctl] = useModals();

	const [everyone, setEveryone] = createSignal(
		((roomConfig() as any)?.mention_everyone ?? true) as boolean,
	);
	const [roles, setRoles] = createSignal(
		((roomConfig() as any)?.mention_roles ?? true) as boolean,
	);

	return (
		<>
			<Submenu
				content={"notifications"}
				onClick={() =>
					modalctl.open({ type: "notifications", room_id: props.room.id })}
			>
				<Item
					onClick={() =>
						setNotifs({
							messages: undefined,
							threads: undefined,
						})}
					classList={{
						selected: roomConfig()?.notifs.messages === undefined &&
							roomConfig()?.notifs.threads === undefined,
					}}
				>
					<div>default</div>
					<div class="subtext">Uses your default notification setting.</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Everything" })}
					classList={{
						selected: roomConfig()?.notifs.messages === "Everything",
					}}
				>
					<div>everything</div>
					<div class="subtext">You will be notified for all messages.</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Watching" })}
					classList={{ selected: roomConfig()?.notifs.messages === "Watching" }}
				>
					<div>watching</div>
					<div class="subtext">
						Messages in this room will show up in your inbox.
					</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Mentions" })}
					classList={{ selected: roomConfig()?.notifs.messages === "Mentions" }}
				>
					<div>mentions</div>
					<div class="subtext">You will only be notified on @mention</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Nothing" })}
					classList={{ selected: roomConfig()?.notifs.messages === "Nothing" }}
				>
					<div>nothing</div>
					<div class="subtext">You won't be notified for anything.</div>
				</Item>
				<Separator />
				<Item
					onClick={() => setNotifs({ threads: undefined })}
					classList={{ selected: roomConfig()?.notifs.threads === undefined }}
				>
					<div>default</div>
					<div class="subtext">
						Uses your default notification setting for threads.
					</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Notify" })}
					classList={{ selected: roomConfig()?.notifs.threads === "Notify" }}
				>
					<div>new threads</div>
					<div class="subtext">You will be notified for new threads.</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Inbox" })}
					classList={{ selected: roomConfig()?.notifs.threads === "Inbox" }}
				>
					<div>threads to inbox</div>
					<div class="subtext">New threads will show up in your inbox.</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Nothing" })}
					classList={{ selected: roomConfig()?.notifs.threads === "Nothing" }}
				>
					<div>ignore threads</div>
					<div class="subtext">You won't be notified for new threads.</div>
				</Item>
				<Separator />
				<CheckboxOption
					id="room-mention-everyone"
					checked={everyone()}
					onChange={(checked) => {
						setEveryone(checked);
						setNotifs({ mention_everyone: checked });
					}}
					seed={`menu-room-${props.room.id}-everyone`}
					style={{ "align-items": "start", gap: "0" }}
				>
					<Checkbox
						checked={everyone()}
						seed={`menu-room-${props.room.id}-everyone`}
					/>
					<label for="room-mention-everyone" style="margin-left: 8px;">
						<div>Enable @everyone and @here</div>
						<div class="dim">
							You will receive notifications when @everyone or @here is
							mentioned.
						</div>
					</label>
				</CheckboxOption>
				<CheckboxOption
					id="room-mention-roles"
					checked={roles()}
					onChange={(checked) => {
						setRoles(checked);
						setNotifs({ mention_roles: checked });
					}}
					seed={`menu-room-${props.room.id}-roles`}
					style={{ "align-items": "start", gap: "0" }}
				>
					<Checkbox
						checked={roles()}
						seed={`menu-room-${props.room.id}-roles`}
					/>
					<label for="room-mention-roles" style="margin-left: 8px;">
						<div>Enable all role mentions</div>
						<div class="dim">
							You will receive notifications when any @role you have is
							mentioned.
						</div>
					</label>
				</CheckboxOption>
			</Submenu>
			<Show
				when={isMuted()}
				fallback={
					<Submenu content={"mute"} onClick={() => setMute(null)}>
						<Item onClick={() => setMute(fifteen_mins)}>for 15 minutes</Item>
						<Item onClick={() => setMute(three_hours)}>for 3 hours</Item>
						<Item onClick={() => setMute(eight_hours)}>for 8 hours</Item>
						<Item onClick={() => setMute(one_day)}>for 1 day</Item>
						<Item onClick={() => setMute(one_week)}>for 1 week</Item>
						<Item onClick={() => setMute(null)}>forever</Item>
					</Submenu>
				}
			>
				<Item onClick={unmute}>
					<div>unmute</div>
					<Show when={roomConfig()?.notifs.mute?.expires_at}>
						<div class="subtext">
							unmutes {timeAgo(
								new Date(Date.parse(roomConfig()!.notifs.mute!.expires_at!)),
							)}
						</div>
					</Show>
				</Item>
			</Show>
		</>
	);
}
