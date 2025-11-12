import { Channel } from "sdk";
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
import iconCamera from "./assets/camera.png";
import iconHeadphones from "./assets/headphones.png";
import iconMic from "./assets/mic.png";
import iconScreenshare from "./assets/screenshare.png";
import iconSettings from "./assets/settings.png";
import iconMusic from "./assets/music.png";
import iconExit from "./assets/exit.png";
import { useApi } from "./api.tsx";
import { ToggleIcon } from "./ToggleIcon.tsx";
import { useVoice } from "./voice-provider.tsx";
import { useConfig } from "./config.tsx";
import { flags } from "./flags.ts";
import { useNavigate } from "@solidjs/router";
import { VoiceDebug } from "./VoiceDebug.tsx";
import { createPopup } from "./popup.tsx";
import { useCtx } from "./context.ts";
import { md } from "./markdown.tsx";
import { getColor } from "./colors.ts";
import { useChannel } from "./channelctx.tsx";

export const Voice = (p: { channel: Channel }) => {
	const config = useConfig();
	const api = useApi();
	const [voice, actions] = useVoice();
	const ctx = useCtx();
	const [ch, chUpdate] = useChannel()!;

	createEffect(on(() => p.channel.id, (tid) => {
		if (!voice.threadId || voice.threadId !== tid) actions.connect(tid);
	}));

	const getName = (uid: string) => {
		const user = api.users.fetch(() => uid);
		const room_member = p.channel.room_id
			? api.room_members.fetch(() => p.channel.room_id!, () => uid)
			: null;
		const rm = room_member?.();
		return (rm?.membership === "Join" && rm.override_name) || user()?.name ||
			uid;
	};

	const getUsersWithoutStreams = () => {
		const hasStream = new Set();
		for (const s of voice.rtc?.streams.values() ?? []) {
			hasStream.add(s.user_id);
		}
		const users = [];
		for (const state of api.voiceStates.values()) {
			if (state.thread_id === p.channel.id && !hasStream.has(state.user_id)) {
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
					<Show when={voice.rtc}>
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
												((voice.rtc?.speaking.get(stream.user_id)?.flags ?? 0) &
													1) === 1,
										}}
										onClick={() =>
											setFocused((s) => (s === stream.id ? null : stream.id))}
									>
										<div class="live">live</div>
										<video
											autoplay
											playsinline
											ref={videoRef!}
											muted
										/>
										<div class="status">
											{getName(stream.user_id)}
										</div>
									</div>
								);
							})(voice.rtc?.streams.get(focused()!))}
						</Show>
						<div class="list">
							<For each={[...voice.rtc!.streams.values()]}>
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
													((voice.rtc?.speaking.get(stream.user_id)?.flags ??
														0) &
														1) === 1,
											}}
											style={{
												display: focused() === stream.id ? "none" : undefined,
											}}
											onClick={() =>
												setFocused((s) => (s === stream.id ? null : stream.id))}
										>
											<div class="live">live</div>
											<video
												autoplay
												playsinline
												ref={videoRef!}
												muted
											/>
											<div class="status">
												{getName(stream.user_id)}
											</div>
										</div>
									);
								}}
							</For>
							<For each={getUsersWithoutStreams()}>
								{(uid) => {
									const user = api.users.fetch(() => uid);
									return (
										<div
											class="stream"
											style={{
												"background-color": getColor(uid),
											}}
										>
											<Show when={user()?.avatar}>
												<img
													src={`${config.cdn_url}/thumb/${user()?.avatar}?size=64`}
													class="avatar"
												/>
											</Show>
											<div class="status">
												{getName(uid)}
											</div>
										</div>
									);
								}}
							</For>
						</div>
					</Show>
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
					>
					</span>
				</Show>
				<Switch>
					<Match when={p.channel.deleted_at}>{" (removed)"}</Match>
					<Match when={p.channel.archived_at}>{" (archived)"}</Match>
				</Switch>
				<div style="flex:1"></div>
				<button
					onClick={toggleChat}
					classList={{ active: isChatOpen() }}
					title="Show chat"
				>
					Chat
				</button>
			</header>
			<div class="bottom">
				<div class="controls">
					<button onClick={actions.toggleDeafened}>
						<ToggleIcon checked={!voice.deafened} src={iconHeadphones} />
					</button>
					<button onClick={actions.toggleCam}>
						<ToggleIcon checked={!voice.cameraHidden} src={iconCamera} />
					</button>
					<button onClick={actions.toggleMic}>
						<ToggleIcon checked={!voice.muted} src={iconMic} />
					</button>
					<button onClick={actions.toggleScreen}>
						<ToggleIcon
							checked={voice.screenshareEnabled}
							src={iconScreenshare}
						/>
					</button>
					<Show when={flags.has("voice_music")}>
						<button onClick={actions.playMusic}>
							<ToggleIcon
								checked={voice.musicPlaying}
								src={iconMusic}
							/>
						</button>
					</Show>
					<button class="disconnect" onClick={actions.disconnect}>
						<img class="icon" src={iconExit} />
					</button>
				</div>
			</div>
		</div>
	);
};

export const VoiceTray = () => {
	const api = useApi();
	const [voice, actions] = useVoice();
	const thread = () =>
		voice.threadId ? api.channels.fetch(() => voice.threadId!)() : null;
	const room = () =>
		thread()?.room_id ? api.rooms.fetch(() => thread()?.room_id!)() : null;
	const user = () => api.users.cache.get("@self");

	const calcConnectedDuration = () => {
		const joinedAt = api.voiceState()?.joined_at;
		if (joinedAt) {
			return (Date.now() - Date.parse(joinedAt));
		} else {
			return (0);
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

	return (
		<div class="voice-tray">
			<Show when={voice.rtc}>
				<div class="row">
					<div style="flex:1;display:flex;align-items:center">
						<button class="status" onClick={openDebug}>
							<Switch>
								<Match when={!voice.rtc}>
									<div class="status disconnected">disconnected</div>
								</Match>
								<Match when={voice.rtc?.state() === "connected"}>
									<div class="status connected">connected</div>
								</Match>
								<Match when={voice.rtc?.state() === "failed"}>
									<div class="status failed">failed</div>
								</Match>
								<Match when={true}>
									<div class="status">{voice.rtc?.state()}</div>
								</Match>
							</Switch>
						</button>
						<div style="width:8px"></div>
						<Duration ms={connectedDuration()} />
					</div>
					<button style="width: auto" onClick={actions.disconnect}>
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
					<button data-tooltip="toggle camera" onClick={actions.toggleCam}>
						<ToggleIcon checked={!voice.cameraHidden} src={iconCamera} />
					</button>
					<button
						data-tooltip="toggle screenshare"
						onClick={actions.toggleScreen}
					>
						<ToggleIcon
							checked={voice.screenshareEnabled}
							src={iconScreenshare}
						/>
					</button>
				</div>
			</Show>
			<div class="row toolbar">
				<div style="flex:1">
					<Show when={user()} fallback="loading...">
						{api.users.cache.get("@self")?.name}
						<Show when={!user()?.registered_at}>
							{" "}
							<b class="dim">(guest)</b>
						</Show>
					</Show>
				</div>
				<button onClick={actions.toggleMic}>
					<ToggleIcon checked={!voice.muted} src={iconMic} />
				</button>
				<button onClick={actions.toggleDeafened}>
					<ToggleIcon checked={!voice.deafened} src={iconHeadphones} />
				</button>
				<button onClick={() => nav("/settings")}>
					<img class="icon" src={iconSettings} />
				</button>
			</div>
			<popup.View />
		</div>
	);
};

const Duration = (props: { ms: number }) => {
	const hours = () => Math.floor(props.ms / (1000 * 60 * 60));
	const mins = () =>
		(Math.floor(props.ms / (1000 * 60)) % 60).toString().padStart(2, "0");
	const secs = () =>
		(Math.floor(props.ms / 1000) % 60).toString().padStart(2, "0");

	return (
		<span class="dim">
			<Show when={hours()}>
				{hours()}
				<span class="">:</span>
			</Show>
			{mins()}
			<span class="">:</span>
			{secs()}
		</span>
	);
};
