import { useNavigate } from "@solidjs/router";
import { createResource, Show } from "solid-js";
import { timeAgo } from "../Time.tsx";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { Item, Menu, Separator, Submenu } from "./Parts.tsx";

// the context menu for rooms
export function RoomMenu(props: { room_id: string }) {
	const ctx = useCtx();
	const api = useApi();
	const nav = useNavigate();
	const room = api.rooms.fetch(() => props.room_id);

	const copyId = () => navigator.clipboard.writeText(props.room_id);

	const copyLink = () => {
		const url = `${ctx.client.opts.apiUrl}/room/${props.room_id}`;
		navigator.clipboard.writeText(url);
	};

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(room())));

	const leave = () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "are you sure you want to leave?",
			cont(confirm) {
				if (!confirm) return;
				ctx.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id: props.room_id,
							user_id: api.users.cache.get("@self")!.id,
						},
					},
				});
			},
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
			<RoomNotificationMenu room_id={props.room_id} />
			<Separator />
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
			<Item onClick={leave}>leave</Item>
			<Separator />
			<Item onClick={copyId}>copy id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}

function RoomNotificationMenu(props: { room_id: string }) {
	const api = useApi();
	const [roomConfig, { mutate }] = createResource(
		() => props.room_id,
		async (room_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/config/room/{room_id}",
				{
					params: { path: { room_id } },
				},
			);
			return data;
		},
	);

	const setNotifs = (notifs: Partial<import("sdk").NotifsRoom>) => {
		const current = roomConfig() ?? { notifs: {}, frontend: {} };
		const newConfig = {
			...current,
			notifs: { ...current.notifs, ...notifs },
		};
		for (const key in newConfig.notifs) {
			if (
				newConfig.notifs[key as keyof typeof newConfig.notifs] === undefined
			) {
				delete newConfig.notifs[key as keyof typeof newConfig.notifs];
			}
		}
		mutate(newConfig);
		api.client.http.PUT("/api/v1/config/room/{room_id}", {
			params: { path: { room_id: props.room_id } },
			body: newConfig,
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
		if (c.notifs.mute.expires_at === null) return true;
		return Date.parse(c.notifs.mute.expires_at) > Date.now();
	};

	const fifteen_mins = 15 * 60 * 1000;
	const three_hours = 3 * 60 * 60 * 1000;
	const eight_hours = 8 * 60 * 60 * 1000;
	const one_day = 24 * 60 * 60 * 1000;
	const one_week = 7 * one_day;

	return (
		<>
			<Submenu content={"notifications"}>
				<Item
					onClick={() =>
						setNotifs({
							messages: undefined,
							mentions: undefined,
							threads: undefined,
						})}
				>
					<div>default</div>
					<div class="subtext">Uses your default notification setting.</div>
				</Item>
				<Item onClick={() => setNotifs({ messages: "Notify" })}>
					<div>everything</div>
					<div class="subtext">You will be notified for all messages.</div>
				</Item>
				<Item onClick={() => setNotifs({ threads: "Notify" })}>
					<div>new threads</div>
					<div class="subtext">You will be notified for new threads.</div>
				</Item>
				<Item
					onClick={() =>
						setNotifs({ messages: "Watching", threads: "Watching" })}
				>
					<div>watching</div>
					<div class="subtext">
						Threads and messages mark this room unread.
					</div>
				</Item>
				<Item
					onClick={() => setNotifs({ messages: "Ignore", mentions: "Notify" })}
				>
					<div>mentions</div>
					<div class="subtext">You will only be notified on @mention</div>
				</Item>
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
