import {
	createContext,
	createEffect,
	createSignal,
	on,
	onCleanup,
	type ParentProps,
	useContext,
} from "solid-js";
import { createStore } from "solid-js/store";
import { useApi } from "@/api";
import { createVoiceClient } from "../components/features/voice/rtc";
import { ReactiveMap } from "@solid-primitives/map";
// @ts-ignore
import vadProcessorUrl from "../components/features/voice/vad-processor?url";
import { useCurrentUser } from "../contexts/currentUser.tsx";
import { colors, logger } from "../logger.ts";

type VoiceClient = ReturnType<typeof createVoiceClient>;

export type VoiceProviderState = {
	muted: boolean;
	deafened: boolean;
	cameraHidden: boolean;
	screenshareEnabled: boolean;
	musicPlaying: boolean;
	rtc: VoiceClient | null;
	threadId: string | null;
	hasVoiceActivity: boolean;
	preferences: ReactiveMap<string, VoiceConfigUser>;
};

type VoiceConfigUser = {
	volume: number;
	mute: boolean;
	mute_video: boolean;
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

const VoiceCtx = createContext<[VoiceProviderState, VoiceActions]>();

export const useVoice = () => useContext(VoiceCtx)!;

const voiceLog = logger.for("voice");
const rtcLog = logger.for("rtc");

export const VoiceProvider = (props: ParentProps) => {
	const api = useApi();
	const vad = createVoiceActivityDetection();
	const [state, update] = createStore<VoiceProviderState>({
		muted: true,
		deafened: false,
		cameraHidden: true,
		screenshareEnabled: false,
		musicPlaying: false,
		rtc: null,
		threadId: null,
		preferences: new ReactiveMap(),
		get hasVoiceActivity() {
			return vad.hasVoiceActivity();
		},
	});

	let streamMic: MediaStream | undefined;
	let streamCam: MediaStream | undefined;

	let screenVidTn: RTCRtpTransceiver | undefined;
	let screenAudTn: RTCRtpTransceiver | undefined;
	let micTn: RTCRtpTransceiver | undefined;
	let camTn: RTCRtpTransceiver | undefined;
	let musicTn: RTCRtpTransceiver | undefined;

	let rtcCreated = false;

	createEffect(on(vad.hasVoiceActivity, (activity) => {
		state.rtc?.sendSpeaking(activity ? 1 : 0);
	}));

	createEffect(() => {
		state.rtc?.updateIndicators({
			self_deaf: state.deafened,
			self_mute: state.muted,
			self_video: !state.cameraHidden,
			self_screen: state.screenshareEnabled,
		});
	});

	api.events.on("sync", async ([e]) => {
		const currentUser = api.users.cache.get("@self");
		const user_id = currentUser?.id;
		if (!user_id) return;

		if (e.type === "VoiceState" && e.user_id === user_id) {
			if (e.state) {
				const rtc = state.rtc;
				if (!rtc) return;

				update("threadId", e.state.channel_id);

				update("muted", e.state.self_mute);
				update("deafened", e.state.self_deaf);
				update("cameraHidden", !e.state.self_video);
				update("screenshareEnabled", !!e.state.screenshare);

				if (!rtcCreated) {
					rtc.createStream("user");
					rtc.createStream("screen");
					rtc.createStream("music");
					rtcCreated = true;
				}

				// if we have an existing microphone stream, use it
				if (streamMic && !state.muted) {
					voiceLog.debug("restore microphone stream");
					if (!micTn) {
						micTn = rtc.createTransceiver("user", "audio");
					}
					const track = streamMic.getAudioTracks()[0];
					if (track) {
						await micTn!.sender.replaceTrack(track);
						micTn!.direction = "sendonly";
					}
				}

				// if we have an existing camera stream, use it
				if (streamCam && !state.cameraHidden) {
					voiceLog.debug("restore camera stream");
					if (!camTn) {
						camTn = rtc.createTransceiver("user", "video");
					}
					const track = streamCam.getVideoTracks()[0];
					if (track) {
						await camTn!.sender.replaceTrack(track);
						camTn!.direction = "sendonly";
					}
				}
			} else {
				rtcLog.debug("our voice state was deleted, cleanup");
				disconnect();
			}
		}
	});

	function disconnect() {
		state.rtc?.disconnect();
		state.rtc?.conn.close();
		update("rtc", null);
		update("threadId", null);
		rtcCreated = false;
	}

	onCleanup(() => {
		rtcLog.debug("cleanup");
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
			rtcLog.debug(`connect to ${threadId}`, state.rtc);
			state.rtc?.connect(threadId);
		},
		disconnect() {
			rtcLog.debug("disconnect");
			disconnect();
		},
		toggleMic: async () => {
			if (!streamMic) {
				// if we don't have a microphone, try to get it
				const stream = await navigator.mediaDevices.getUserMedia({
					audio: true,
				})
					.catch(handleGetMediaError);
				if (stream) {
					voiceLog.debug("got microphone stream", stream);
					streamMic = stream;
					update("muted", false);
					if (state.rtc) {
						if (!micTn) {
							micTn = state.rtc.createTransceiver("user", "audio");
						}
						voiceLog.debug("got microphone stream", stream);
						const track = streamMic.getAudioTracks()[0];
						if (track) {
							if (micTn.currentDirection !== "stopped") {
								await micTn.sender.replaceTrack(track);
								micTn.direction = "sendonly";
							} else {
								voiceLog.warn("microphone transceiver is stopped", micTn);
							}
						}
					}
					vad.connect(streamMic);
				} else {
					voiceLog.warn("couldn't get microphone stream");
				}
			} else {
				if (state.rtc) {
					if (!micTn) {
						micTn = state.rtc.createTransceiver("user", "audio");
					}
					const tr = micTn.sender.track;
					if (tr) {
						voiceLog.debug("toggle microphone track enabled");
						tr.enabled = state.muted;
						update("muted", !state.muted);
					} else if (streamMic && state.muted) {
						voiceLog.debug("restore microphone track");
						const track = streamMic.getAudioTracks()[0];
						if (!track) {
							throw new Error("microphone doesn't have any audio tracks?");
						}
						await micTn.sender.replaceTrack(track);
						micTn.direction = "sendonly";
						track.enabled = true;
						update("muted", false);
					} else {
						voiceLog.debug("toggle microphone muted");
						update("muted", !state.muted);
					}
				} else {
					voiceLog.debug("toggle microphone muted, not connected to rtc");
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
					voiceLog.debug("got camera stream", stream);
					streamCam = stream;
					update("cameraHidden", false);
					if (state.rtc) {
						if (!camTn) {
							camTn = state.rtc.createTransceiver("user", "video");
						}
						voiceLog.debug("got camera stream", stream);
						const track = streamCam.getVideoTracks()[0];
						if (track) {
							await camTn.sender.replaceTrack(track);
							camTn.direction = "sendonly";
						}
					}
				} else {
					voiceLog.warn("couldn't get camera stream");
				}
			} else {
				if (state.rtc) {
					if (!camTn) {
						camTn = state.rtc.createTransceiver("user", "video");
					}
					const tr = camTn.sender.track;
					if (tr) {
						voiceLog.debug("toggle camera track enabled");
						tr.enabled = state.cameraHidden;
						update("cameraHidden", !state.cameraHidden);
					} else if (streamCam && state.cameraHidden) {
						voiceLog.debug("restore camera track");
						const track = streamCam.getVideoTracks()[0];
						if (!track) {
							throw new Error("camera doesn't have any video tracks?");
						}
						await camTn.sender.replaceTrack(track);
						camTn.direction = "sendonly";
						track.enabled = true;
						update("cameraHidden", false);
					} else {
						voiceLog.debug("toggle camera hidden");
						update("cameraHidden", !state.cameraHidden);
					}
				} else {
					voiceLog.debug("toggle camera hidden, not connected to rtc");
					update("cameraHidden", !state.cameraHidden);
				}
			}
		},
		toggleScreen: async () => {
			if (!state.rtc) return;
			if (!screenVidTn) {
				screenVidTn = state.rtc.createTransceiver("screen", "video");
			}
			if (!screenAudTn) {
				screenAudTn = state.rtc.createTransceiver("screen", "audio");
			}
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
			// TEMP: music playing is for debugging, since its easier than yelling into the microphone every time i want to test webrtc
			if (!state.rtc) return;
			if (!musicTn) {
				musicTn = state.rtc.createTransceiver("music", "audio");
			}
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

// TODO: investigate more ways to debounce
// deep neural network: https://www.microsoft.com/en-us/research/wp-content/uploads/2017/04/Tashev-Mirsamadi_DNN-based-Causal-VAD.pdf
// another implementation: https://github.com/snakers4/silero-vad
function createVoiceActivityDetection() {
	const vadLog = logger.for("vad");
	vadLog.debug("init");

	const [hasVoiceActivity, setHasVoiceActivity] = createSignal(false);
	const ctx = new AudioContext();
	let source: MediaStreamAudioSourceNode | undefined;
	let node: AudioWorkletNode | undefined;

	const initWorklet = async () => {
		try {
			await ctx.audioWorklet.addModule(vadProcessorUrl);
			node = new AudioWorkletNode(ctx, "vad-processor");
			node.port.onmessage = (event) => {
				if (event.data && typeof event.data.hasVoiceActivity === "boolean") {
					setHasVoiceActivity(event.data.hasVoiceActivity);
				}
			};
			if (source) {
				source.connect(node);
			}
		} catch (e) {
			vadLog.error("failed to initialize audio worklet", e);
		}
	};

	initWorklet();

	onCleanup(() => {
		vadLog.debug("cleanup");
		node?.disconnect();
		source?.disconnect();
		ctx.close();
	});

	return {
		hasVoiceActivity,
		connect(stream: MediaStream) {
			source?.disconnect();
			source = ctx.createMediaStreamSource(stream);
			if (node) {
				source.connect(node);
			}
			if (ctx.state === "suspended") {
				ctx.resume();
			}
			vadLog.debug("new stream connected");
		},
	};
}
