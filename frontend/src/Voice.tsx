import { Thread } from "sdk";
import {
	createContext,
	createEffect,
	createSignal,
	For,
	Match,
	onCleanup,
	Show,
	Switch,
	useContext,
	createMemo,
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

type VoiceSettings = {
	muted: boolean;
	deafened: boolean;
	cameraHidden: boolean;
};
const VoiceControls = createContext<
	[VoiceSettings, SetStoreFunction<VoiceSettings>]
>();

const useVoiceControls = () => useContext(VoiceControls)!;

export const Voice = (p: { thread: Thread }) => {
	const api = useApi();
	const rtc = createVoiceClient();

	const [voiceSettings, updateVoiceSettings] = createStore({
		muted: true,
		deafened: false,
		cameraHidden: true,
	});

	// TEMP: debugging
	(globalThis as any).rtc = rtc.conn;

	let screenVidTn: RTCRtpTransceiver;
	let screenAudTn: RTCRtpTransceiver;
	let micTn: RTCRtpTransceiver;
	let camTn: RTCRtpTransceiver;
	let musicTn: RTCRtpTransceiver;

	const [muted, setMuted] = createSignal(true);
	const [deafened, setDeafened] = createSignal(false);
	const [cameraHidden, setCameraHidden] = createSignal(true);

	api.events.on("sync", (e) => {
		const user_id = api.users.cache.get("@self")!.id;
		if (
			e.type === "VoiceState" && e.user_id === user_id && e.state &&
			!screenVidTn
		) {
			rtc.createStream("user");
			rtc.createStream("screen");
			rtc.createStream("music");
			micTn = rtc.createTransceiver("user", "audio");
			camTn = rtc.createTransceiver("user", "video");
			screenAudTn = rtc.createTransceiver("screen", "audio");
			screenVidTn = rtc.createTransceiver("screen", "video");
			musicTn = rtc.createTransceiver("music", "audio");
		}
	});

	const playMusic = async () => {
		const tr = musicTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
		} else {
			const audio = document.createElement("audio");
			audio.src = "https://dump.celery.eu.org/resoundingly-one-bullsnake.opus";
			audio.crossOrigin = "anonymous";
			await new Promise((res) =>
				audio.addEventListener("loadedmetadata", res, { once: true })
			);

			const stream: MediaStream = "captureStream" in audio
				? (audio as any).captureStream()
				: (audio as any).mozCaptureStream();
			const track = stream.getAudioTracks()[0];
			await musicTn.sender.replaceTrack(track);
			musicTn.direction = "sendonly";
			console.log("playing music with transceiver", musicTn);
			audio.play();
		}
	};

	const toggleScreen = async () => {
		const tr = screenVidTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
			const t = screenAudTn.sender.track;
			if (t) t.enabled = !tr.enabled;
			// this version of disabling a track is more aggressive and will re-prompt when toggling
			// tr.stop();
			// await screenTn.sender.replaceTrack(null);
			// screenTn.direction = "inactive";
		} else {
			const stream = await navigator.mediaDevices.getDisplayMedia({
				video: true,
				audio: true,
			}).catch(handleGetMediaError);
			if (!stream) return;

			{
				const tr = stream.getVideoTracks()[0];
				if (!tr) {
					console.warn("no video track");
					return;
				}
				await screenVidTn.sender.replaceTrack(tr);
				screenVidTn.direction = "sendonly";
			}

			{
				const tr = stream.getAudioTracks()[0];
				if (tr) {
					console.warn("no audio track");
				} else {
					await screenAudTn.sender.replaceTrack(tr);
					screenAudTn.direction = "sendonly";
				}
			}
		}
	};

	const toggleMic = async () => {
		const tr = micTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
			setMuted(!tr.enabled);
		} else {
			const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
				.catch(handleGetMediaError);
			if (!stream) return;

			const track = stream.getAudioTracks()[0];
			if (!track) {
				console.warn("no track");
				return;
			}

			await micTn.sender.replaceTrack(track);
			micTn.direction = "sendonly";
			setMuted(false);
		}
	};

	const toggleCam = async () => {
		const tr = camTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
			setCameraHidden(!tr.enabled);
		} else {
			const stream = await navigator.mediaDevices.getUserMedia({ video: true })
				.catch(handleGetMediaError);
			if (!stream) return;

			const track = stream.getVideoTracks()[0];
			if (!track) {
				console.warn("no track");
				return;
			}

			await camTn.sender.replaceTrack(track);
			camTn.direction = "sendonly";
			setCameraHidden(false);
		}
	};

	function handleGetMediaError(e: Error) {
		switch (e.name) {
			case "NotFoundError":
				alert("no camera, microphone, display was found");
				break;
			case "SecurityError":
			case "PermissionDeniedError":
				// do nothing; this is the same as the user canceling the call
				break;
			default:
				alert(`error opening media: ${e.message}`);
				break;
		}
	}

	async function debug() {
		console.group("[rtc:debug] debug stats");
		const stats = await rtc.conn.getStats();
		for (const [_, stat] of [...stats]) {
			console.log(stat);
		}
		console.groupEnd();
	}

	rtc.connect(p.thread.id);

	const room = api.rooms.fetch(() => p.thread.room_id);

	const getName = (uid: string) => {
		const user = api.users.fetch(() => uid);
		const room_member = p.thread.room_id ? api.room_members.fetch(() => p.thread.room_id!, () => uid) : null;
		const rm = room_member?.();
		return (rm?.membership === "Join" && rm.override_name) || user()?.name || uid;
	}

	const getUsersWithoutStreams = () => {
		const hasStream = new Set();
		for (const s of rtc.streams.values()) {
			hasStream.add(s.user_id)
		}
		const users = [];
		for (const state of api.voiceStates.values()) {
			if (state.thread_id === p.thread.id && !hasStream.has(state.user_id)) {
				users.push(state.user_id)
			}
		}
		return users
	};

	return (
		<VoiceControls.Provider value={[voiceSettings, updateVoiceSettings]}>
			<div class="webrtc">
				<div class="bottom">
					<div class="controls">
						<button onClick={toggleCam}>toggle cam</button>
						<button onClick={toggleMic}>toggle mic</button>
						<button onClick={toggleScreen}>toggle screen</button>
						<button onClick={playMusic}>music</button>
						<div>participants: {api.voiceStates.size}</div>
					</div>
				</div>
				<div class="streams">
					<For each={[...rtc.streams.values()]}>
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
										muted={deafened()}
									/>
								</div>
							)
						}}
					</For>
					<For each={getUsersWithoutStreams()}>{(uid) => {
						return (
							<div class="stream">
								{getName(uid)}
							</div>
						)
					}}
					</For>
				</div>
				<div class="tray">
					<div class="row">
						<div style="flex:1">
							<div
								style={rtc.state() === "connected"
									? "color:green"
									: "color:yellow"}
							>
								{rtc.state()}
							</div>
						</div>
						<Show when={false}>
							{/* TODO: stay connected when navigating away from voice channels */}
							{/* TODO: allow being disconnected while focused on a voice channel */}
							<div>
								<button>
									{/* disconnect */}
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
							<button data-tooltip="toggle camera" onClick={toggleCam}>
								{/* camera */}
								<ToggleIcon checked={cameraHidden()} src={iconCamera} />
							</button>
							<button data-tooltip="toggle screenshare" onClick={toggleScreen}>
								{/* screenshare */}
								<img class="icon" src={iconScreenshare} />
							</button>
						</div>
					</div>
					<div class="row toolbar">
						<div style="flex:1">{api.users.cache.get("@self")?.name}</div>
						<button onClick={toggleMic}>
							{/* mute */}
							<ToggleIcon checked={muted()} src={iconMic} />
						</button>
						<button onClick={() => setDeafened((d) => !d)}>
							{/* deafen */}
							<ToggleIcon checked={deafened()} src={iconHeadphones} />
						</button>
						<button onClick={() => alert("todo")}>
							{/* settings */}
							<img class="icon" src={iconSettings} />
						</button>
					</div>
				</div>
			</div>
		</VoiceControls.Provider>
	);
};

export const VoiceTray = (p: { thread: Thread }) => {
	return (
		<div class="voice-tray">
			<div class="row">
				<div style="flex:1">
					<div
						style={rtc.state() === "connected"
							? "color:green"
							: "color:yellow"}
					>
						{rtc.state()}
					</div>
				</div>
				<Show when={false}>
					{/* TODO: stay connected when navigating away from voice channels */}
					{/* TODO: allow being disconnected while focused on a voice channel */}
					<div>
						<button>
							{/* disconnect */}
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
					<button data-tooltip="toggle camera" onClick={toggleCam}>
						{/* camera */}
						<ToggleIcon checked={cameraHidden()} src={iconCamera} />
					</button>
					<button data-tooltip="toggle screenshare" onClick={toggleScreen}>
						{/* screenshare */}
						<img class="icon" src={iconScreenshare} />
					</button>
				</div>
			</div>
			<div class="row toolbar">
				<div style="flex:1">{api.users.cache.get("@self")?.name}</div>
				<button onClick={toggleMic}>
					{/* mute */}
					<ToggleIcon checked={muted()} src={iconMic} />
				</button>
				<button onClick={() => setDeafened((d) => !d)}>
					{/* deafen */}
					<ToggleIcon checked={deafened()} src={iconHeadphones} />
				</button>
				<button onClick={() => alert("todo")}>
					{/* settings */}
					<img class="icon" src={iconSettings} />
				</button>
			</div>
		</div>
	);
}
