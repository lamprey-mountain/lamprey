import { Thread } from "sdk";
import {
	createEffect,
	createSignal,
	For,
	Match,
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
import { useApi } from "./api.tsx";
import { ToggleIcon } from "./ToggleIcon.tsx";
import { useVoice } from "./voice-provider.tsx";

export const Voice = (p: { thread: Thread }) => {
	const api = useApi();
	const [voice, actions] = useVoice();

	if (!voice.threadId) actions.connect(p.thread.id);

	const getName = (uid: string) => {
		const user = api.users.fetch(() => uid);
		const room_member = p.thread.room_id
			? api.room_members.fetch(() => p.thread.room_id!, () => uid)
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
			if (state.thread_id === p.thread.id && !hasStream.has(state.user_id)) {
				users.push(state.user_id);
			}
		}
		return users;
	};

	const [focused, setFocused] = createSignal<null | string>(null);
	const [controls, setControls] = createSignal(true);

	let controlsTimeout: NodeJS.Timeout;
	const showControls = () => {
		setControls(true);
		clearTimeout(controlsTimeout);
		controlsTimeout = setTimeout(() => setControls(false), 3000);
	};

	return (
		<div class="webrtc" onMouseMove={showControls}>
			<div class="streams">
				<Show when={voice.rtc}>
					<For each={[...voice.rtc!.streams.values()]}>
						{(stream) => {
							let videoRef!: HTMLVideoElement;
							createEffect(() => {
								if (videoRef) videoRef.srcObject = stream.media;
							});
							return (
								<div
									class="stream"
									classList={{ fullscreen: focused() === stream.id }}
									onClick={() =>
										setFocused((s) => s === stream.id ? null : stream.id)}
								>
									<div class="live">live</div>
									<video
										autoplay
										playsinline
										ref={videoRef!}
										muted
									/>
								</div>
							);
						}}
					</For>
					<For each={getUsersWithoutStreams()}>
						{(uid) => {
							return <div class="stream">{getName(uid)}</div>;
						}}
					</For>
				</Show>
			</div>
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
					<button onClick={actions.playMusic}>
						<ToggleIcon
							checked={voice.musicPlaying}
							src={iconMusic}
						/>
					</button>
				</div>
			</div>
		</div>
	);
};

export const VoiceTray = () => {
	const api = useApi();
	const [voice, actions] = useVoice();
	const thread = voice.threadId
		? api.threads.fetch(() => voice.threadId!)
		: () => null;
	const room = thread()?.room_id
		? api.rooms.fetch(() => thread()?.room_id!)
		: () => null;

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

	return (
		<div class="voice-tray">
			<Show when={voice.rtc}>
				<div class="row">
					<div style="flex:1;display:flex;align-items:center">
						<Switch>
							<Match when={!voice.rtc}>
								<div class="status disconnected">disconnected</div>
							</Match>
							<Match when={voice.rtc?.state() === "connected"}>
								<div class="status connected">connected</div>
							</Match>
							<Match when={true}>
								<div class="status">{voice.rtc?.state()}</div>
							</Match>
						</Switch>
						<div style="width:8px"></div>
						<Duration ms={connectedDuration()} />
					</div>
					<button onClick={actions.disconnect}>disconnect</button>
				</div>
				<div class="row">
					<div>
						<Show when={room()} fallback={thread()?.name}>
							{room()?.name} / {thread()?.name}
						</Show>
					</div>
					<div style="flex:1"></div>
					<div>
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
				</div>
			</Show>
			<div class="row toolbar">
				<div style="flex:1">{api.users.cache.get("@self")?.name}</div>
				<button onClick={actions.toggleMic}>
					<ToggleIcon checked={!voice.muted} src={iconMic} />
				</button>
				<button onClick={actions.toggleDeafened}>
					<ToggleIcon checked={!voice.deafened} src={iconHeadphones} />
				</button>
				<button onClick={() => alert("todo")}>
					<img class="icon" src={iconSettings} />
				</button>
			</div>
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
