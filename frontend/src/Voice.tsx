import { Thread } from "sdk";
import {
	createContext,
	createEffect,
	createMemo,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
	useContext,
} from "solid-js";
import iconCamera from "./assets/camera.png";
import iconHeadphones from "./assets/headphones.png";
import iconMic from "./assets/mic.png";
import iconScreenshare from "./assets/screenshare.png";
import iconSettings from "./assets/settings.png";
import iconX from "./assets/x.png";
import { useApi } from "./api.tsx";
import { ToggleIcon } from "./ToggleIcon.tsx";
import { createVoiceClient } from "./rtc.ts";
import { createStore, SetStoreFunction } from "solid-js/store";
import { useVoice } from "./voice.tsx";

export const Voice = (p: { thread: Thread }) => {
	const api = useApi();
	const [voice, actions] = useVoice();
	const rtc = createVoiceClient();

	// TEMP: debugging
	(globalThis as any).rtc = rtc.conn;

	console.log("set rtc");
	actions.setVoiceClient(rtc);
	console.log("connect");
	rtc.connect(p.thread.id);
	onCleanup(() => {
		rtc.disconnect();
		actions.setVoiceClient(null);
	});

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
		for (const s of rtc.streams.values()) {
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

	return (
		<div class="webrtc">
			<div class="streams">
				<Show when={voice.rtc}>
					<For each={[...voice.rtc!.streams.values()]}>
						{(stream) => {
							let videoRef!: HTMLVideoElement;
							createEffect(() => {
								if (videoRef) videoRef.srcObject = stream.media;
							});
							return (
								<div class="stream">
									<video
										autoplay
										playsinline
										ref={videoRef!}
										muted={voice.deafened}
									/>
								</div>
							);
						}}
					</For>
				</Show>
				<For each={getUsersWithoutStreams()}>
					{(uid) => {
						return <div class="stream">{getName(uid)}</div>;
					}}
				</For>
			</div>
			<div class="bottom">
				<div class="controls">
					<button onClick={actions.toggleCam}>toggle cam</button>
					<button onClick={actions.toggleMic}>toggle mic</button>
					<button onClick={actions.toggleScreen}>toggle screen</button>
					<button onClick={actions.playMusic}>music</button>
					<div>participants: {api.voiceStates.size}</div>
				</div>
			</div>
		</div>
	);
};

export const VoiceTray = (p: { thread: Thread }) => {
	const api = useApi();
	const [voice, actions] = useVoice();

	const room = api.rooms.fetch(() => p.thread.room_id!);

	return (
		<div class="voice-tray">
			<div class="row">
				<div style="flex:1">
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
				</div>
				<Show when={false}>
					<div>
						<button>
							<img class="icon" src={iconX} />
						</button>
					</div>
				</Show>
			</div>
			<div class="row">
				<div>
					<Show when={room()} fallback={p.thread.name}>
						{room()?.name} / {p.thread.name}
					</Show>
				</div>
				<div style="flex:1"></div>
				<div>
					<button data-tooltip="toggle camera" onClick={actions.toggleCam}>
						<ToggleIcon checked={voice.cameraHidden} src={iconCamera} />
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
			<div class="row toolbar">
				<div style="flex:1">{api.users.cache.get("@self")?.name}</div>
				<button onClick={actions.toggleMic}>
					<ToggleIcon checked={voice.muted} src={iconMic} />
				</button>
				<button onClick={actions.toggleDeafened}>
					<ToggleIcon checked={voice.deafened} src={iconHeadphones} />
				</button>
				<button onClick={() => alert("todo")}>
					<img class="icon" src={iconSettings} />
				</button>
			</div>
		</div>
	);
};
