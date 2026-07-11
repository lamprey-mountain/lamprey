import { A, useNavigate } from "@solidjs/router";
import type { Channel } from "sdk";
import {
	createEffect,
	createSignal,
	For,
	Match,
	on,
	onCleanup,
	Show,
	Switch,
} from "solid-js";
import { useApi, useChannels, useRooms } from "@/api";
import { createPopup } from "@/app/popup";
import {
	icCamera,
	icCancel,
	icExit,
	icHeadphones,
	icMic,
	icScreenshare,
	icSettings,
} from "@/utils/icons";
import { Duration } from "@/atoms/Duration.tsx";
import { Icon } from "@/atoms/Icon";
import { ToggleIcon } from "@/atoms/ToggleIcon.tsx";
import { AvatarWithStatus } from "@/components/shared/User";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { useVoice } from "../features/voice/context";
import { VoiceDebug } from "../features/voice/VoiceDebug";
import { ChannelT, UserT } from "@/types";

// TODO: add tooltips for icon buttons
// TODO: move voice parts to a separate component(?)

export const UserTray = () => {
	const nav = useNavigate();
	const channels = useChannels();
	const rooms = useRooms();

	const currentUser = useCurrentUser();
	const [voice, voiceActions] = useVoice();

	const openUserProfile = () => {
		// TODO: add popout
	};

	const voiceDebugPopup = createPopup({
		title: () => "webrtc debug",
		content: () => <VoiceDebug onClose={voiceDebugPopup.hide} />,
	});

	const openVoiceDebug = () => {
		if (voiceDebugPopup.visible()) {
			voiceDebugPopup.hide();
		} else {
			voiceDebugPopup.show();
		}
	};

	const voiceDuration = useVoiceDuration();
	const [voiceChannel, setVoiceChannel] = createSignal(null as ChannelT | null);
	// FIXME: make .use() reactive with undefined
	// const voiceChannel = channels.use(() => voice.joinedChannelId ?? undefined);
	const voiceRoom = rooms.use(() => voiceChannel()?.room_id ?? undefined);

	// FIXME: don't automatically reconnect when doing voiceActions.disconnect while navigated to /channel/{voice_channel_id}
	createEffect(() => {
		const id = voice.joinedChannelId;
		if (id) {
			setVoiceChannel(channels.cache.get(id) ?? null);
		} else {
			setVoiceChannel(null);
		}
	});

	return (
		<div class="user-tray">
			<Show when={voiceChannel()}>
				{(chan) => (
					<>
						<Show when={voice.screensharing}>
							<div class="row screenshare-row">
								<div class="screenshare-info">
									{/* TODO: display more info */}
									sharing screen
								</div>
								<button
									type="button"
									class="button icon-button"
									data-tooltip="stop screenshare"
									onClick={() => voiceActions.stopScreenshare()}
								>
									<Icon src={icCancel} />
								</button>
							</div>
						</Show>
						<div class="row voice-row">
							<div class="voice-info">
								<button
									type="button"
									class="voice-status"
									onClick={openVoiceDebug}
								>
									<Switch>
										<Match
											when={
												voice.connectionState === "connecting" ||
												voice.connectionState === "pending"
											}
										>
											<div class="status disconnected">disconnected</div>
										</Match>
										<Match when={voice.connectionState === "connected"}>
											<div class="status connected">connected</div>
										</Match>
										<Match when={voice.connectionState === "disconnected"}>
											<div class="status failed">disconnected (failed)</div>
										</Match>
									</Switch>
								</button>
								<div class="voice-location">
									in{" "}
									<A href={`/channel/${chan().id}`}>
										<Show when={voiceRoom()}>
											{(room) => (
												<>
													{room().name}
													{" / "}
												</>
											)}
										</Show>
										{chan().name}
									</A>{" "}
									for <Duration ms={voiceDuration.duration} />
								</div>
							</div>

							<menu class="voice-toolbar">
								<button
									type="button"
									class="button icon-button"
									data-tooltip="toggle camera"
									onClick={() => voiceActions.toggleCamera()}
								>
									<ToggleIcon checked={!voice.camera} src={icCamera} />
								</button>
								<button
									type="button"
									class="button icon-button"
									data-tooltip="toggle screenshare"
									onClick={() => voiceActions.toggleScreenshare()}
								>
									<ToggleIcon
										checked={voice.screensharing}
										src={icScreenshare}
									/>
								</button>
								<button
									type="button"
									class="button icon-button"
									onClick={voiceActions.disconnect}
								>
									<Icon src={icExit} />
								</button>
							</menu>
						</div>
					</>
				)}
			</Show>
			<div class="row user-row">
				<Show when={currentUser()}>
					{(u) => (
						<div class="current-user" onClick={openUserProfile}>
							<AvatarWithStatus user={u()} />
							<div class="info">
								<div class="name">
									{u().name}
									<Show when={!u().registered_at}>
										{" "}
										<b class="dim">(guest)</b>
									</Show>
								</div>
								<UserPresenceActivity user={u()} />
							</div>
						</div>
					)}
				</Show>
				<menu class="user-toolbar">
					<button
						type="button"
						class="button icon-button"
						onClick={() => voiceActions.toggleMicrophone()}
					>
						<ToggleIcon checked={!voice.muted} src={icMic} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={() => voiceActions.toggleDeafened()}
					>
						<ToggleIcon checked={!voice.deafened} src={icHeadphones} />
					</button>
					<button
						type="button"
						class="button icon-button"
						onClick={() => nav("/settings")}
					>
						<Icon src={icSettings} />
					</button>
				</menu>
			</div>
			<voiceDebugPopup.View />
		</div>
	);
};

const UserPresenceActivity = (props: { user: UserT }) => {
	const getText = () => {
		for (const a of props.user.presence.activities) {
			if (a.type === "Custom") return a.text;
		}
	};

	return (
		<Show when={getText()}>
			{(t) => <div class="user-activity">{t()}</div>}
		</Show>
	);
};

const useVoiceDuration = () => {
	const api = useApi();
	const [duration, setDuration] = createSignal<number | null>(0);

	const interval = setInterval(() => {
		const joinedAt = api.voiceState?.joined_at;
		if (joinedAt) {
			setDuration(Date.now() - Date.parse(joinedAt));
		} else {
			setDuration(null);
		}
	}, 100);

	onCleanup(() => {
		clearInterval(interval);
	});

	return {
		get duration() {
			return duration();
		},
	};
};
