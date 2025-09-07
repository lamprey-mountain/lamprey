import {
	createContext,
	onCleanup,
	type ParentProps,
	useContext,
} from "solid-js";
import { createStore } from "solid-js/store";
import { useApi } from "./api";
import { createVoiceClient } from "./rtc";

type VoiceClient = ReturnType<typeof createVoiceClient>;

export type VoiceState = {
	muted: boolean;
	deafened: boolean;
	cameraHidden: boolean;
	screenshareEnabled: boolean;
	musicPlaying: boolean;
	rtc: VoiceClient | null;
	threadId: string | null;
};

export type VoiceActions = {
	connect: (threadId: string) => void;
	disconnect: () => void;
	toggleMic: () => Promise<void>;
	toggleCam: () => Promise<void>;
	toggleScreen: () => Promise<void>;
	playMusic: () => Promise<void>;
	toggleDeafened: () => void;
};

const VoiceCtx = createContext<[VoiceState, VoiceActions]>();

export const useVoice = () => useContext(VoiceCtx)!;

export const VoiceProvider = (props: ParentProps) => {
	const api = useApi();
	const [state, update] = createStore<VoiceState>({
		muted: true,
		deafened: false,
		cameraHidden: true,
		screenshareEnabled: false,
		musicPlaying: false,
		rtc: null,
		threadId: null,
	});

	let streamMic: MediaStream | undefined;
	let streamCam: MediaStream | undefined;

	let screenVidTn: RTCRtpTransceiver | undefined;
	let screenAudTn: RTCRtpTransceiver | undefined;
	let micTn: RTCRtpTransceiver | undefined;
	let camTn: RTCRtpTransceiver | undefined;
	let musicTn: RTCRtpTransceiver | undefined;

	api.events.on("sync", async (e) => {
		const user_id = api.users.cache.get("@self")!.id;
		if (
			e.type === "VoiceState" && e.user_id === user_id && e.state
		) {
			const rtc = state.rtc;
			if (!rtc) return;
			rtc.createStream("user");
			rtc.createStream("screen");
			rtc.createStream("music");
			micTn = rtc.createTransceiver("user", "audio");
			camTn = rtc.createTransceiver("user", "video");
			screenAudTn = rtc.createTransceiver("screen", "audio");
			screenVidTn = rtc.createTransceiver("screen", "video");
			musicTn = rtc.createTransceiver("music", "audio");

			// if we have an existing microphone stream, use it
			if (streamMic && !state.muted) {
				console.log("[voice] restore microphone stream");
				const track = streamMic.getAudioTracks()[0];
				if (track) {
					await micTn.sender.replaceTrack(track);
					micTn.direction = "sendonly";
				}
			}

			// if we have an existing camera stream, use it
			if (streamCam && !state.cameraHidden) {
				console.log("[voice] restore camera stream");
				const track = streamCam.getVideoTracks()[0];
				if (track) {
					await camTn.sender.replaceTrack(track);
					camTn.direction = "sendonly";
				}
			}
		}
	});

	onCleanup(() => {
		const rtc = state.rtc;
		if (!rtc) return;
		rtc.disconnect();
	});

	const actions: VoiceActions = {
		connect(threadId) {
			if (!state.rtc) {
				update("rtc", createVoiceClient());
			}
			update("threadId", threadId);
			state.rtc?.connect(threadId);
		},
		disconnect() {
			state.rtc?.disconnect();
			state.rtc?.conn.close();
			update("rtc", null);
			update("threadId", null);
		},
		toggleMic: async () => {
			if (!streamMic) {
				// if we don't have a microphone, try to get it
				const stream = await navigator.mediaDevices.getUserMedia({
					audio: true,
				})
					.catch(handleGetMediaError);
				if (stream) {
					console.log("[voice] got microphone stream", stream);
					streamMic = stream;
					update("muted", false);
					if (state.rtc && micTn) {
						console.log("[voice] got microphone stream", stream);
						const track = streamMic.getAudioTracks()[0];
						if (track) {
							await micTn.sender.replaceTrack(track);
							micTn.direction = "sendonly";
						}
					}
				} else {
					console.warn("[voice] couldn't get microphone stream");
				}
			} else {
				if (state.rtc && micTn) {
					const tr = micTn.sender.track;
					if (tr) {
						console.log("[voice] toggle microphone track enabled");
						tr.enabled = state.muted;
						update("muted", !state.muted);
					} else if (streamMic && state.muted) {
						console.log("[voice] restore microphone track");
						const track = streamMic.getAudioTracks()[0];
						if (!track) {
							throw new Error("microphone doesn't have any audio tracks?");
						}
						await micTn.sender.replaceTrack(track);
						micTn.direction = "sendonly";
						track.enabled = true;
						update("muted", false);
					} else {
						console.log("[voice] toggle microphone muted");
						update("muted", !state.muted);
					}
				} else {
					console.log("[voice] toggle microphone muted, not connected to rtc");
					update("muted", !state.muted);
				}
			}
		},
		toggleCam: async () => {
			if (!streamCam) {
				// if we don't have a camera, try to get it
				const stream = await navigator.mediaDevices.getUserMedia({
					video: true,
				})
					.catch(handleGetMediaError);
				if (stream) {
					console.log("[voice] got camera stream", stream);
					streamCam = stream;
					update("cameraHidden", false);
					if (state.rtc && camTn) {
						console.log("[voice] got camera stream", stream);
						const track = streamCam.getVideoTracks()[0];
						if (track) {
							await camTn.sender.replaceTrack(track);
							camTn.direction = "sendonly";
						}
					}
				} else {
					console.warn("[voice] couldn't get camera stream");
				}
			} else {
				if (state.rtc && camTn) {
					const tr = camTn.sender.track;
					if (tr) {
						console.log("[voice] toggle camera track enabled");
						tr.enabled = state.cameraHidden;
						update("cameraHidden", !state.cameraHidden);
					} else if (streamCam && state.cameraHidden) {
						console.log("[voice] restore camera track");
						const track = streamCam.getVideoTracks()[0];
						if (!track) {
							throw new Error("camera doesn't have any video tracks?");
						}
						await camTn.sender.replaceTrack(track);
						camTn.direction = "sendonly";
						track.enabled = true;
						update("cameraHidden", false);
					} else {
						console.log("[voice] toggle camera hidden");
						update("cameraHidden", !state.cameraHidden);
					}
				} else {
					console.log("[voice] toggle camera hidden, not connected to rtc");
					update("cameraHidden", !state.cameraHidden);
				}
			}
		},
		toggleScreen: async () => {
			if (!state.rtc || !screenVidTn || !screenAudTn) return;
			const tr = screenVidTn.sender.track;
			if (tr) {
				tr.enabled = !tr.enabled;
				const t = screenAudTn.sender.track;
				if (t) t.enabled = !tr.enabled;
				update("screenshareEnabled", tr.enabled);
			} else {
				const stream = await navigator.mediaDevices.getDisplayMedia({
					video: true,
					audio: true,
				}).catch(handleGetMediaError);
				if (!stream) return;
				const videoTrack = stream.getVideoTracks()[0];
				if (videoTrack) {
					await screenVidTn.sender.replaceTrack(videoTrack);
					screenVidTn.direction = "sendonly";
				}
				const audioTrack = stream.getAudioTracks()[0];
				if (audioTrack) {
					await screenAudTn.sender.replaceTrack(audioTrack);
					screenAudTn.direction = "sendonly";
				}
				update("screenshareEnabled", true);
			}
		},
		playMusic: async () => {
			if (!state.rtc || !musicTn) return;
			const tr = musicTn.sender.track;
			if (tr) {
				tr.enabled = !tr.enabled;
				update("musicPlaying", tr.enabled);
			} else {
				const audio = document.createElement("audio");
				audio.src =
					"https://dump.celery.eu.org/resoundingly-one-bullsnake.opus";
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
				audio.play();
				update("musicPlaying", true);
			}
		},
		toggleDeafened: () => {
			update("deafened", (d) => !d);
		},
	};

	return (
		<VoiceCtx.Provider value={[state, actions]}>
			{props.children}
		</VoiceCtx.Provider>
	);
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
