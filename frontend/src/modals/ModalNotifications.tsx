import type {
	Channel,
	NotifsChannel,
	NotifsRoom,
	PreferencesChannel,
	PreferencesRoom,
} from "sdk";
import { createMemo, createSignal, Show } from "solid-js";
import { useApi, useChannels, useRooms } from "@/api";
import { useCtx } from "@/app/context";
import { CheckboxOption } from "@/atoms/CheckboxOption";
import { Dropdown } from "@/atoms/Dropdown";
import { formatDuration } from "@/atoms/Duration";
import { Checkbox } from "@/atoms/icons";
import { isMuteActive } from "@/lib/notifications/preferences";
import { getDate } from "@/utils/general";
import { Modal } from "./mod";

type NotifsMessages = NonNullable<NotifsChannel["messages"]>;
type NotifsThreads = NonNullable<NotifsChannel["threads"]>;

const DEFAULT_CHANNEL_FRONTEND: PreferencesChannel["frontend"] = {
	grid_view: false,
	show_all_threads: false,
	show_people_without_video: false,
};

const MUTE_DURATIONS = [
	{ key: "15m", ms: 15 * 60 * 1000 },
	{ key: "3h", ms: 3 * 60 * 60 * 1000 },
	{ key: "8h", ms: 8 * 60 * 60 * 1000 },
	{ key: "1d", ms: 24 * 60 * 60 * 1000 },
	{ key: "1w", ms: 7 * 24 * 60 * 60 * 1000 },
] as const;

const MUTE_KEYS = ["unmute", "forever", ...MUTE_DURATIONS.map((d) => d.key)];

const MESSAGE_VALUES: Array<NotifsMessages | "default"> = [
	"default",
	"Everything",
	"Watching",
	"Mentions",
	"Nothing",
];

const THREAD_VALUES: Array<NotifsThreads | "default"> = [
	"default",
	"Notify",
	"Inbox",
	"Nothing",
];

type ModalNotificationsProps = {
	room_id: string;
};

export const ModalNotifications = (props: ModalNotificationsProps) => {
	return (
		<Modal>
			<div class="modal-notifications">
				<RoomNotifications room_id={props.room_id} />
				<ChannelNotifications room_id={props.room_id} />
			</div>
		</Modal>
	);
};

function RoomNotifications(props: ModalNotificationsProps) {
	const ctx = useCtx();
	const { t } = ctx;
	const api = useApi();
	const rooms = useRooms();
	const roomsList = rooms.use(() => props.room_id);

	const room = () => roomsList();
	const config = () => room()?.preferences;
	const notifs = () => config()?.notifs;

	const setNotifs = (notifs: Partial<NotifsRoom>) => {
		const r = room();
		if (!r) return;

		const merged: NotifsRoom = {
			mention_everyone: true,
			mention_roles: true,
			...(r.preferences?.notifs ?? {}),
			...notifs,
		};
		for (const key in merged) {
			if (merged[key as keyof NotifsRoom] === undefined) {
				delete merged[key as keyof NotifsRoom];
			}
		}

		const newConfig: PreferencesRoom = {
			notifs: merged,
			frontend: r.preferences?.frontend ?? {},
			privacy: r.preferences?.privacy ?? {
				dms: true,
				friends: true,
				rpc: true,
				exif: true,
			},
		};

		rooms.cache.set(r.id, { ...r, preferences: newConfig });
		api.preferences.setRoomPreferences(r.id, newConfig).catch((e) => {
			console.error("Failed to update room preferences", e);
		});
	};

	const setMute = (key: string) => {
		if (key === "unmute") {
			setNotifs({ mute: undefined });
			return;
		}

		const duration = MUTE_DURATIONS.find((d) => d.key === key);
		const expires_at = duration
			? new Date(Date.now() + duration.ms).toISOString()
			: null;
		setNotifs({ mute: { expires_at, duration: duration?.ms ?? null } });
	};

	const muteDetail = () => {
		const mute = notifs()?.mute;
		if (!isMuteActive(mute)) return undefined;
		if (!mute?.expires_at) return t("notifs.forever");
		const remaining = +getDate(mute.expires_at) - Date.now();
		return t("notifs.for_duration", { duration: formatDuration(remaining) });
	};

	const everyone = () => notifs()?.mention_everyone ?? true;
	const roles = () => notifs()?.mention_roles ?? true;

	const messageLabel = (value: NotifsMessages | "default") => {
		if (value === "default") {
			const fallback = ctx.preferences().notifs.messages.toLowerCase();
			return `${t("notifs.default")} (${fallback})`;
		}
		return value.toLowerCase();
	};

	const threadLabel = (value: NotifsThreads | "default") => {
		if (value === "default") {
			const fallback = ctx.preferences().notifs.threads.toLowerCase();
			return `${t("notifs.default")} (${fallback})`;
		}
		if (value === "Notify") return t("notifs.new_threads");
		if (value === "Inbox") return t("notifs.threads_to_inbox");
		return t("notifs.ignore_threads");
	};

	return (
		<Show when={room()}>
			{(r) => (
				<>
					<h3>{t("notifs.title")}</h3>
					<h3 class="dim">{r().name}</h3>
					<div>
						<h3 class="dim">{t("notifs.mute")}</h3>
						<div style="display:flex;align-items:center;gap:8px;">
							<Dropdown
								placeholder={muteDetail() ?? t("notifs.mute")}
								onSelect={(value) => value && setMute(value)}
								options={MUTE_KEYS.map((key) => {
									const duration = MUTE_DURATIONS.find((d) => d.key === key);
									return {
										item: key,
										label:
											key === "unmute"
												? t("notifs.unmute")
												: key === "forever"
													? t("notifs.forever")
													: t("notifs.for_duration", {
															duration: formatDuration(duration?.ms ?? 0),
														}),
									};
								})}
							/>
						</div>
					</div>
					<div>
						<h3 class="dim">messages</h3>
						<Dropdown<NotifsMessages | "default">
							selected={notifs()?.messages ?? "default"}
							onSelect={(value) =>
								value &&
								setNotifs({
									messages: value === "default" ? undefined : value,
								})
							}
							options={MESSAGE_VALUES.map((value) => ({
								item: value,
								label: messageLabel(value),
							}))}
						/>
					</div>
					<div>
						<h3 class="dim">threads</h3>
						<Dropdown<NotifsThreads | "default">
							selected={notifs()?.threads ?? "default"}
							onSelect={(value) =>
								value &&
								setNotifs({
									threads: value === "default" ? undefined : value,
								})
							}
							options={THREAD_VALUES.map((value) => ({
								item: value,
								label: threadLabel(value),
							}))}
						/>
					</div>
					<div>
						<h3 class="dim">special notifications</h3>
						<CheckboxOption
							id="opt-everyone"
							checked={everyone()}
							onChange={(checked) => setNotifs({ mention_everyone: checked })}
							seed={`modal-notifications-${props.room_id}-everyone`}
						>
							<Checkbox
								checked={everyone()}
								seed={`modal-notifications-${props.room_id}-everyone`}
							/>
							<label for="opt-everyone">
								<div>{t("notifs.mention_everyone")}</div>
								<div class="dim">
									{t("notifs.mention_everyone_description")}
								</div>
							</label>
						</CheckboxOption>
						<CheckboxOption
							id="opt-role"
							checked={roles()}
							onChange={(checked) => setNotifs({ mention_roles: checked })}
							seed={`modal-notifications-${props.room_id}-role`}
						>
							<Checkbox
								checked={roles()}
								seed={`modal-notifications-${props.room_id}-role`}
							/>
							<label for="opt-role">
								<div>{t("notifs.mention_roles")}</div>
								<div class="dim">{t("notifs.mention_roles_description")}</div>
							</label>
						</CheckboxOption>
						{/* TODO: (when impl'd) mobile push notifications */}
						{/* TODO: (when impl'd) calendar event */}
					</div>
				</>
			)}
		</Show>
	);
}

function ChannelNotifications(props: ModalNotificationsProps) {
	const ctx = useCtx();
	const { t } = ctx;
	const api = useApi();
	const channels = useChannels();
	const rooms = useRooms();
	const [selectedId, setSelectedId] = createSignal<string>();

	const roomChannels = createMemo(() =>
		Array.from(channels.cache.values())
			.filter(
				(c) =>
					c.room_id === props.room_id &&
					c.type !== "Category" &&
					!c.parent_id &&
					!c.deleted_at,
			)
			.sort((a, b) => (a.position ?? 0) - (b.position ?? 0)),
	);

	const fallbackChannel = () =>
		roomChannels().find((c) => c.type === "Text") ?? roomChannels()[0];

	const channel = () =>
		roomChannels().find((c) => c.id === selectedId()) ?? fallbackChannel();

	const notifs = () => channel()?.preferences?.notifs;

	const setNotifs = (notifs: Partial<NotifsChannel>) => {
		const c = channel();
		if (!c) return;

		const merged: NotifsChannel = {
			...(c.preferences?.notifs ?? {}),
			...notifs,
		};
		for (const key in merged) {
			if (merged[key as keyof NotifsChannel] === undefined) {
				delete merged[key as keyof NotifsChannel];
			}
		}

		const newConfig: PreferencesChannel = {
			notifs: merged,
			frontend: c.preferences?.frontend ?? DEFAULT_CHANNEL_FRONTEND,
		};

		channels.cache.set(c.id, { ...c, preferences: newConfig });
		api.preferences.setChannelPreferences(c.id, newConfig).catch((e) => {
			console.error("Failed to update channel preferences", e);
		});
	};

	// clears all overrides, inheriting from the room again
	const resetNotifs = () => {
		const c = channel();
		if (!c) return;

		const newConfig: PreferencesChannel = {
			notifs: {},
			frontend: c.preferences?.frontend ?? DEFAULT_CHANNEL_FRONTEND,
		};

		channels.cache.set(c.id, { ...c, preferences: newConfig });
		api.preferences.setChannelPreferences(c.id, newConfig).catch((e) => {
			console.error("Failed to update channel preferences", e);
		});
	};

	const setMuted = (muted: boolean) => {
		setNotifs({
			mute: muted ? { expires_at: null, duration: null } : undefined,
		});
	};

	const defaultMessages = () =>
		rooms.get(props.room_id)?.preferences?.notifs.messages ??
		ctx.preferences().notifs.messages;

	const hasOverrides = () => {
		const n = notifs();
		if (!n) return false;
		return (
			n.mute != null ||
			n.messages != null ||
			n.replies != null ||
			n.threads != null
		);
	};

	const muteLabel = () =>
		isMuteActive(notifs()?.mute) ? t("notifs.unmute") : t("notifs.mute");

	const channelLabel = (c: Channel) => c.name || t("notifs.title");

	return (
		<div>
			<h3 class="dim">channel settings</h3>
			<Show
				when={roomChannels().length}
				fallback={<div class="dim">no channels in this room</div>}
			>
				<div style="display:flex;flex-direction:column;gap:8px;">
					<Dropdown
						selected={channel()?.id ?? ""}
						onSelect={(value) => value && setSelectedId(value)}
						options={roomChannels().map((c) => ({
							item: c.id,
							label: `#${channelLabel(c)}`,
						}))}
					/>
					<Show when={channel()}>
						{(c) => (
							<div style="display:flex;background:#aaa2;align-items:center;justify-content:space-between;padding:0 4px;gap:8px;">
								<div>#{channelLabel(c())}</div>
								<div style="display:flex;align-items:center;gap:8px;">
									<Dropdown<NotifsMessages | "default">
										selected={notifs()?.messages ?? "default"}
										onSelect={(value) =>
											value &&
											setNotifs({
												messages: value === "default" ? undefined : value,
											})
										}
										options={MESSAGE_VALUES.map((value) => ({
											item: value,
											label:
												value === "default"
													? `${t("notifs.default")} (${defaultMessages().toLowerCase()})`
													: value.toLowerCase(),
										}))}
									/>
									<button
										type="button"
										class="button"
										onClick={() => setMuted(!isMuteActive(notifs()?.mute))}
									>
										{muteLabel()}
									</button>
									<Show when={hasOverrides()}>
										<button
											type="button"
											class="button danger"
											onClick={resetNotifs}
										>
											reset
										</button>
									</Show>
								</div>
							</div>
						)}
					</Show>
				</div>
			</Show>
		</div>
	);
}
