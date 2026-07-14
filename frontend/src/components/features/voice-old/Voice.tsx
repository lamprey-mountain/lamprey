import { useNavigate } from "@solidjs/router";
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
import iconCamera from "@/assets/camera.png";
import iconExit from "@/assets/exit.png";
import iconHeadphones from "@/assets/headphones.png";
import iconMic from "@/assets/mic.png";
import iconMusic from "@/assets/music.png";
import iconScreenshare from "@/assets/screenshare.png";
import iconSettings from "@/assets/settings.png";
import { Icon } from "@/atoms/Icon";
import { ToggleIcon } from "@/atoms/ToggleIcon.tsx";
import { AvatarWithStatus } from "@/components/shared/User";
import { useChannel } from "@/contexts/channel";
import { useCurrentUser } from "@/contexts/currentUser.tsx";
import { getColor } from "@/lib/colors";
import { flags } from "@/lib/flags";
import { md } from "@/lib/markdown";
import { useVoice } from "../voice/context.tsx";
import { VoiceDebug } from "./VoiceDebug.tsx";

export const Voice = (p: { channel: Channel }) => {
	const api = useApi();
	const [voice, actions] = useVoice();
	const [ch, chUpdate] = useChannel()!;

	// FIXME: this seems to be very janky
	createEffect(
		on(
			() => p.channel.id,
			(tid) => {
				if (!voice.joinedChannelId || voice.joinedChannelId !== tid)
					actions.selectChannel(tid);
			},
		),
	);

	const getName = (uid: string) => {
		const room_member = p.channel.room_id
			? api.room_members.use(() => `${p.channel.room_id}!:${uid}`)
			: null;
		return room_member?.()?.override_name || uid;
	};

	const getUsersWithoutStreams = () => {
		const hasStream = new Set();
		for (const s of voice.vc.streams.values() ?? []) {
			hasStream.add(s.user_id);
		}
		const users = [];
		for (const state of api.voiceStates.values()) {
			if (
				(state as any).thread_id === p.channel.id &&
				!hasStream.has(state.user_id)
			) {
				users.push(state.user_id);
			}
		}
		return users;
	};

	const [focused, setFocused] = createSignal<null | string>(null);
	const [controls, setControls] = createSignal(true);

	let controlsTimeout: NodeJS.Timeout = setTimeout(
		() => setControls(false),
		2000,
	);

	const showControls = () => {
		setControls(true);
		clearTimeout(controlsTimeout);
		controlsTimeout = setTimeout(() => setControls(false), 2000);
	};

	const hideControls = () => {
		setControls(false);
		clearTimeout(controlsTimeout);
	};

	onCleanup(() => {
		clearTimeout(controlsTimeout);
	});

	const isChatOpen = () => ch.voice_chat_sidebar_open;
	const toggleChat = () => {
		chUpdate("voice_chat_sidebar_open", (o) => !o);
	};

	return (
		<div
			class="webrtc"
			classList={{ controls: controls(), "stream-focused": !!focused() }}
			onMouseMove={showControls}
			onMouseOut={hideControls}
		>
			<div class="streams">
				<div class="centered">
					<Show when={focused()}>
						{((stream) => {
							if (!stream) return;
							let videoRef!: HTMLVideoElement;
							createEffect(() => {
								if (videoRef) videoRef.srcObject = stream.media;
							});
							return (
								<div
									class="stream"
									classList={{
										fullscreen: focused() === stream.id,
										speaking:
											((voice.vc.speaking.users.get(stream.user_id)?.flags ??
												0) &
												1) ===
											1,
									}}
									onClick={() =>
										setFocused((s) => (s === stream.id ? null : stream.id))
									}
								>
									<div class="live">live</div>
									<video autoplay playsinline ref={videoRef!} muted />
									<div class="status">{getName(stream.user_id)}</div>
								</div>
							);
						})(voice.vc.streams.get(focused()!))}
					</Show>
					<div class="list">
						<For each={[...(voice.vc.streams.values() ?? [])]}>
							{(stream) => {
								let videoRef!: HTMLVideoElement;
								createEffect(() => {
									if (videoRef) videoRef.srcObject = stream.media;
								});

								return (
									<div
										class="stream"
										classList={{
											speaking:
												((voice.vc.speaking.users.get(stream.user_id)?.flags ??
													0) &
													1) ===
												1,
										}}
										style={{
											display: focused() === stream.id ? "none" : undefined,
										}}
										onClick={() =>
											setFocused((s) => (s === stream.id ? null : stream.id))
										}
									>
										<div class="live">live</div>
										<video autoplay playsinline ref={videoRef!} muted />
										<div class="status">{getName(stream.user_id)}</div>
									</div>
								);
							}}
						</For>
						<For each={getUsersWithoutStreams()}>
							{(uid) => {
								const user = api.users.use(() => uid);
								return (
									<div
										class="stream"
										style={{
											"background-color": getColor(uid),
										}}
									>
										<Show when={user}>
											<AvatarWithStatus user={user()} />
											<div class="status">{getName(uid)}</div>
										</Show>
									</div>
								);
							}}
						</For>
					</div>
				</div>
			</div>
			<header class="top">
				<b>{p.channel.name}</b>
				<Show when={p.channel.description}>
					<span class="dim" style="white-space:pre;font-size:1em">
						{"  -  "}
					</span>
					<span
						class="markdown"
						innerHTML={md(p.channel.description ?? "") as string}
					></span>
				</Show>
				<Switch>
					<Match when={p.channel.deleted_at}>{" (removed)"}</Match>
					<Match when={p.channel.archived_at}>{" (archived)"}</Match>
				</Switch>
				<div style="flex:1"></div>
				<button
					type="button"
					class="button"
					onClick={toggleChat}
					classList={{ active: isChatOpen() }}
					title="Show chat"
				>
					Chat
				</button>
			</header>
			<div class="bottom">
				<div class="controls">
					<button
						type="button"
						class="button"
						onClick={() => actions.toggleDeafened()}
					>
						<ToggleIcon enabled={!voice.deafened} src={iconHeadphones} />
					</button>
					<button
						type="button"
						class="button"
						onClick={() => actions.toggleCamera()}
					>
						<ToggleIcon enabled={voice.camera} src={iconCamera} />
					</button>
					<button
						type="button"
						class="button"
						onClick={() => actions.toggleMicrophone()}
					>
						<ToggleIcon enabled={!voice.muted} src={iconMic} />
					</button>
					<button
						type="button"
						class="button"
						onClick={actions.toggleScreenshare}
					>
						<ToggleIcon enabled={voice.screensharing} src={iconScreenshare} />
					</button>
					<Show when={flags.has("voice_music")}>
						<button type="button" class="button" onClick={actions.playMusic}>
							<ToggleIcon enabled={voice.musicing} src={iconMusic} />
						</button>
					</Show>
					<button type="button" class="disconnect" onClick={actions.disconnect}>
						<Icon src={iconExit} />
					</button>
				</div>
			</div>
		</div>
	);
};

export const VoiceTray = () => {
	const api2 = useApi();
	const channels2 = useChannels();
	const rooms2 = useRooms();
	const currentUser = useCurrentUser();
	const [voice, actions] = useVoice();
	const threadData = voice.joinedChannelId
		? channels2.use(() => voice.joinedChannelId!)
		: null;
	const thread = () => threadData?.();
	const roomData = () => {
		const t = thread();
		return t?.room_id ? rooms2.use(() => t.room_id!) : null;
	};
	const room = () => roomData()?.();

	const calcConnectedDuration = () => {
		const joinedAt = api2.voiceState?.joined_at;
		if (joinedAt) {
			return Date.now() - Date.parse(joinedAt);
		} else {
			return 0;
		}
	};

	const [connectedDuration, setConnectedDuration] = createSignal(
		calcConnectedDuration(),
	);

	const interval = setInterval(() => {
		setConnectedDuration(calcConnectedDuration());
	}, 100);

	onCleanup(() => {
		clearInterval(interval);
	});

	const nav = useNavigate();

	const popup = createPopup({
		title: () => "webrtc debug",
		content: () => <VoiceDebug onClose={popup.hide} />,
	});

	const openDebug = () => {
		if (popup.visible()) {
			popup.hide();
		} else {
			popup.show();
		}
	};

	// FIXME: types
	return (
		<div class="voice-tray">
			<Show
				when={
					voice.vc.connectionState() !== "disconnected" &&
					voice.vc.connectionState()
				}
			>
				{(cs) => (
					<>
						<div class="row">
							<div style="flex:1;display:flex;align-items:center">
								<button type="button" class="status" onClick={openDebug}>
									<Switch>
										<Match when={cs() === "connecting" || cs() === "pending"}>
											<div class="status disconnected">disconnected</div>
										</Match>
										<Match when={cs() === "connected"}>
											<div class="status connected">connected</div>
										</Match>
										<Match when={false}>
											<div class="status failed">failed</div>
										</Match>
										<Match when={true}>
											<div class="status">{cs()}</div>
										</Match>
									</Switch>
								</button>
								<div style="width:8px"></div>
								<Duration ms={connectedDuration()} />
							</div>
							<button
								type="button"
								class="button"
								style="width: auto"
								onClick={actions.disconnect}
							>
								disconnect
							</button>
						</div>
						<div class="row">
							<div>
								in{" "}
								<a href={`/thread/${thread()?.id}`}>
									<Show when={room()} fallback={thread()?.name}>
										{room()?.name} / {thread()?.name}
									</Show>
								</a>
							</div>
							<div style="flex:1"></div>
							<button
								type="button"
								class="button"
								data-tooltip="toggle camera"
								onClick={actions.toggleCam}
							>
								<ToggleIcon enabled={!voice.cameraHidden} src={iconCamera} />
							</button>
							<button
								type="button"
								class="button"
								data-tooltip="toggle screenshare"
								onClick={actions.toggleScreen}
							>
								<ToggleIcon
									enabled={voice.screenshareEnabled}
									src={iconScreenshare}
								/>
							</button>
						</div>
					</>
				)}
			</Show>
			<div class="row toolbar">
				<AvatarWithStatus user={currentUser()!} />
				<div style="flex:1">
					<Show when={currentUser()} fallback="loading...">
						{currentUser()?.name}
						<Show when={!currentUser()?.registered_at}>
							{" "}
							<b class="dim">(guest)</b>
						</Show>
					</Show>
				</div>
				<button type="button" class="button" onClick={actions.toggleMic}>
					<ToggleIcon enabled={!voice.muted} src={iconMic} />
				</button>
				<button type="button" class="button" onClick={actions.toggleDeafened}>
					<ToggleIcon enabled={!voice.deafened} src={iconHeadphones} />
				</button>
				<button type="button" class="button" onClick={() => nav("/settings")}>
					<Icon src={iconSettings} />
				</button>
			</div>
			<popup.View />
		</div>
	);
};
