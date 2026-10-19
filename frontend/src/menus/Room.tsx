import type { NotifsRoom, PreferencesRoom, Room } from "sdk";
import { Show } from "solid-js";
import { useApi, useRooms } from "@/api";
import { useCtx } from "@/app/context";
import { formatDuration } from "@/atoms/Duration";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Checkbox } from "@/atoms/icons";
import { timeAgo } from "@/atoms/Time.tsx";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useModals } from "@/contexts/modal";
import { useNavigate } from "@/contexts/router.tsx";
import { usePermissions } from "@/hooks/usePermissions.ts";
import { flags } from "@/lib/flags";
import { getDate } from "@/utils/general.tsx";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";

// the context menu for rooms
export function RoomMenu(props: { room_id: string }) {
	const ctx = useCtx();
	const rooms2 = useRooms();
	const nav = useNavigate();
	const room = rooms2.use(() => props.room_id);
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
					query: { soft: false },
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
			<Item onClick={() => rooms2.markRead(props.room_id)}>mark as read</Item>
			<Item onClick={copyLink}>copy link</Item>
			<Item
				onClick={() =>
					modalctl.open({
						type: "privacy",
						room_id: props.room_id,
					})
				}
			>
				privacy
			</Item>
			<Show when={hasPermission("InviteCreate")}>
				<Item
					onClick={() =>
						modalctl.open({
							type: "invite_create",
							room_id: props.room_id,
						})
					}
				>
					create invite
				</Item>
			</Show>
			<Show when={room()}>{(r) => <RoomNotificationMenu room={r()} />}</Show>
			<Separator />
			<Show when={hasPermission("ChannelManage")}>
				<Item
					onClick={() => {
						modalctl.open({
							type: "channel_create",
							room_id: props.room_id,
						});
					}}
				>
					create channel
				</Item>
			</Show>
			<Submenu content={"edit"} onClick={settings("")}>
				<Item onClick={settings("")}>info</Item>
				<Show when={hasPermission("AnalyticsView")}>
					<Item onClick={settings("/analytics")}>analytics</Item>
				</Show>
				<Show when={hasPermission("EmojiManage")}>
					<Item onClick={settings("/emoji")}>emoji</Item>
				</Show>
				<Separator />
				<Show when={hasPermission("IntegrationsManage")}>
					<Item onClick={settings("/bots")}>bots</Item>
					<Item onClick={settings("/webhooks")}>webhooks</Item>
				</Show>
				<Separator />
				<Show when={hasPermission("InviteManage")}>
					<Item onClick={settings("/invites")}>invites</Item>
				</Show>
				<Show when={hasPermission("RoleManage")}>
					<Item onClick={settings("/roles")}>roles</Item>
				</Show>
				<Item onClick={settings("/members")}>members</Item>
				<Separator />
				<Show when={hasPermission("RoomManage") && flags.has("automod")}>
					<Item onClick={settings("/automod")}>automod</Item>
				</Show>
				<Show when={hasPermission("MemberBan")}>
					<Item onClick={settings("/bans")}>bans</Item>
				</Show>
				<Show when={hasPermission("AuditLogView")}>
					<Item onClick={settings("/logs")}>audit log</Item>
				</Show>
			</Submenu>
			<Item onClick={leave} color="danger">
				leave
			</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}

function RoomNotificationMenu(props: { room: Room }) {
	const ctx = useCtx();
	const { t } = ctx;
	const api2 = useApi();
	const rooms2 = useRooms();
	const roomConfig = () => props.room.preferences;

	const setNotifs = (notifs: Partial<NotifsRoom>) => {
		const current = roomConfig();
		const newConfig: PreferencesRoom = {
			notifs: {
				mention_everyone: true,
				mention_roles: true,
				...(current?.notifs ?? {}),
				...notifs,
			},
			frontend: current?.frontend ?? {},
			privacy: current?.privacy ?? {
				dms: true,
				friends: true,
				rpc: true,
				exif: true,
			},
		};
		rooms2.cache.set(props.room.id, {
			...props.room,
			preferences: newConfig,
		});
		api2.preferences.setRoomPreferences(props.room.id, newConfig).catch((e) => {
			console.error("Failed to update room preferences", e);
		});
	};

	const setMute = (duration_ms: number | null) => {
		const expires_at =
			duration_ms === null
				? null
				: new Date(Date.now() + duration_ms).toISOString();
		setNotifs({ mute: { expires_at } });
	};

	const unmute = () => setNotifs({ mute: undefined });

	const isMuted = () => {
		const c = roomConfig();
		if (!c?.notifs.mute) return false;
		if (!c.notifs.mute.expires_at) return true;
		return getDate(c.notifs.mute.expires_at) > Date.now();
	};

	const fifteen_mins = 15 * 60 * 1000;
	const three_hours = 3 * 60 * 60 * 1000;
	const eight_hours = 8 * 60 * 60 * 1000;
	const one_day = 24 * 60 * 60 * 1000;
	const one_week = 7 * one_day;

	const [, modalctl] = useModals();

	const everyone = () => roomConfig()?.notifs.mention_everyone ?? true;
	const roles = () => roomConfig()?.notifs.mention_roles ?? true;

	return (
		<>
			<Submenu
				content={t("notifs.title")}
				onClick={() =>
					modalctl.open({ type: "notifications", room_id: props.room.id })
				}
			>
				<Item
					onClick={() => setNotifs({ messages: undefined })}
					classList={{
						selected: roomConfig()?.notifs.messages === undefined,
					}}
				>
					<div>{t("notifs.default")}</div>
					<div class="subtext">
						{t("notifs.default_messages_user", {
							value: ctx.preferences().notifs.messages.toLowerCase(),
						})}
					</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Everything" })}
					classList={{
						selected: roomConfig()?.notifs.messages === "Everything",
					}}
				>
					<div>{t("notifs.everything")}</div>
					<div class="subtext">{t("notifs.everything_room")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Watching" })}
					classList={{ selected: roomConfig()?.notifs.messages === "Watching" }}
				>
					<div>{t("notifs.watching")}</div>
					<div class="subtext">{t("notifs.watching_room")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Mentions" })}
					classList={{ selected: roomConfig()?.notifs.messages === "Mentions" }}
				>
					<div>{t("notifs.mentions")}</div>
					<div class="subtext">{t("notifs.mentions_description")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Nothing" })}
					classList={{ selected: roomConfig()?.notifs.messages === "Nothing" }}
				>
					<div>{t("notifs.nothing")}</div>
					<div class="subtext">{t("notifs.nothing_description")}</div>
				</Item>
				<Separator />
				<Item
					onClick={() => setNotifs({ threads: undefined })}
					classList={{ selected: roomConfig()?.notifs.threads === undefined }}
				>
					<div>{t("notifs.default")}</div>
					<div class="subtext">
						{t("notifs.default_threads_user", {
							value: ctx.preferences().notifs.threads.toLowerCase(),
						})}
					</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Notify" })}
					classList={{ selected: roomConfig()?.notifs.threads === "Notify" }}
				>
					<div>{t("notifs.new_threads")}</div>
					<div class="subtext">{t("notifs.new_threads_room")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Inbox" })}
					classList={{ selected: roomConfig()?.notifs.threads === "Inbox" }}
				>
					<div>{t("notifs.threads_to_inbox")}</div>
					<div class="subtext">{t("notifs.threads_to_inbox_description")}</div>
				</Item>
				<Item
					onClick={() => setNotifs({ threads: "Nothing" })}
					classList={{ selected: roomConfig()?.notifs.threads === "Nothing" }}
				>
					<div>{t("notifs.ignore_threads")}</div>
					<div class="subtext">{t("notifs.ignore_threads_description")}</div>
				</Item>
				<Separator />
				<CheckboxOption
					id="room-mention-everyone"
					checked={everyone()}
					onChange={(checked) => {
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
						<div>{t("notifs.mention_everyone")}</div>
						<div class="dim">{t("notifs.mention_everyone_description")}</div>
					</label>
				</CheckboxOption>
				<CheckboxOption
					id="room-mention-roles"
					checked={roles()}
					onChange={(checked) => {
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
						<div>{t("notifs.mention_roles")}</div>
						<div class="dim">{t("notifs.mention_roles_description")}</div>
					</label>
				</CheckboxOption>
			</Submenu>
			<Show
				when={isMuted()}
				fallback={
					<Submenu content={t("notifs.mute")} onClick={() => setMute(null)}>
						<Item onClick={() => setMute(fifteen_mins)}>
							{t("notifs.for_duration", {
								duration: formatDuration(fifteen_mins),
							})}
						</Item>
						<Item onClick={() => setMute(three_hours)}>
							{t("notifs.for_duration", {
								duration: formatDuration(three_hours),
							})}
						</Item>
						<Item onClick={() => setMute(eight_hours)}>
							{t("notifs.for_duration", {
								duration: formatDuration(eight_hours),
							})}
						</Item>
						<Item onClick={() => setMute(one_day)}>
							{t("notifs.for_duration", { duration: formatDuration(one_day) })}
						</Item>
						<Item onClick={() => setMute(one_week)}>
							{t("notifs.for_duration", { duration: formatDuration(one_week) })}
						</Item>
						<Item onClick={() => setMute(null)}>{t("notifs.forever")}</Item>
					</Submenu>
				}
			>
				<Item onClick={unmute}>
					<div>{t("notifs.unmute")}</div>
					<Show when={roomConfig()?.notifs.mute?.expires_at}>
						<div class="subtext">
							{t("notifs.unmutes", {
								time: timeAgo(
									getDate(roomConfig()?.notifs.mute?.expires_at!),
								),
							})}
						</div>
					</Show>
				</Item>
			</Show>
		</>
	);
}
