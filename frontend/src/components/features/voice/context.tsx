import { ReactiveMap } from "@solid-primitives/map";
import {
	createContext,
	type ParentProps,
	useContext,
	createEffect,
	on,
} from "solid-js";
import { createStore } from "solid-js/store";
import { useApi } from "@/api";
import { logger } from "@/utils/logger";
import { DeviceManager } from "./DeviceManager";
import { VoiceClient, type VoiceConnectionState } from "./VoiceClient";
import { createVAD } from "./vad";

type VoiceActions = {
	selectChannel: (channelId: string) => Promise<void>;
	disconnect: () => void;
	toggleMicrophone: (enabled?: boolean) => Promise<void>;
	toggleCamera: (enabled?: boolean) => Promise<void>;
	toggleScreenshare: () => Promise<void>;
	startScreenshare: () => Promise<void>;
	stopScreenshare: () => Promise<void>;
	toggleDeafened: (enabled?: boolean) => Promise<void>;
	playMusic: () => Promise<void>;
	// subscribeToParticipant: (
	// 	userId: string,
	// 	trackKey: "user" | "screen",
	// 	subscribe: boolean,
	// ) => void;
};

type VoiceProviderState = {
	vc: VoiceClient;
	connectionState: VoiceConnectionState;
	joinedChannelId: string | null;
	muted: boolean;
	deafened: boolean;
	camera: boolean;
	screensharing: boolean;
	musicing: boolean;
	hasVoiceActivity: boolean;
	preferences: ReactiveMap<string, VoiceConfigUser>;
};

type VoiceConfigUser = {
	volume: number;
	mute: boolean;
	mute_video: boolean;
};

const VoiceContext = createContext<[VoiceProviderState, VoiceActions]>();

const voiceLog = logger.for("voice");

export const VoiceProvider = (props: ParentProps<{}>) => {
	const api = useApi();
	const vc = new VoiceClient(api);
	const devices = new DeviceManager();
	const vad = createVAD();

	const [store, update] = createStore<VoiceProviderState>({
		vc,
		joinedChannelId: null,
		muted: true,
		deafened: false,
		camera: false,
		screensharing: false,
		musicing: false,
		get hasVoiceActivity() {
			return vad.hasVoiceActivity();
		},
		get connectionState() {
			return vc.connectionState();
		},
		preferences: new ReactiveMap(),
	});

	api.events.on("sync", ([sync, _envelope]) => {
		if (sync.type === "VoiceDispatch") {
			vc.handleSignalingEvent(sync.payload);
		} else if (sync.type === "VoiceState") {
			vc.handleVoiceState(sync.user_id, sync.state ?? null);
		}
	});

	api.events.on("ready", () => {
		vc.drainSendQueue();
	});

	createEffect(
		on(
			() => [store.muted, store.deafened, store.camera] as const,
			([mute, deaf, video]) => {
				const vs = api.voiceState;
				if (!vs) return;
				api.client.send({
					type: "VoiceDispatch",
					channel_id: vs.channel_id,
					command: {
						type: "VoiceState",
						state: {
							channel_id: vs.channel_id,
							self_deaf: deaf,
							self_mute: mute,
							self_video: video,
						},
					},
				});
			},
		),
	);

	const actions: VoiceActions = {
		async selectChannel(channelId: string) {
			vc.connect(channelId);
			update("joinedChannelId", channelId);
		},

		disconnect() {
			vc.disconnect();
			update("joinedChannelId", null);
			update("muted", false);
			update("deafened", false);
			update("camera", false);
			update("screensharing", false);
			update("musicing", false);
		},

		async toggleMicrophone(enabled_?: boolean) {
			// get microphone
			const mic = await devices.acquireMicrophone();

			// enable/disable it as indicated
			const enabled = enabled_ ?? !mic.track.enabled;
			mic.track.enabled = enabled;

			// update muted state
			update("muted", !enabled);

			// connect to vad
			vad.connect(mic.stream);

			if (vc.connectionState() === "disconnected") return;

			// connect to transceiver
			const tr = vc.acquireTransceiver("user", "audio");
			if (tr.currentDirection !== "stopped") {
				await tr.sender.replaceTrack(mic.track);
				tr.direction = "sendonly";
			} else {
				voiceLog.warn("microphone transceiver is stopped", tr);
			}
		},

		async toggleCamera(enabled_?: boolean) {
			const cam = await devices.acquireCamera();
			const enabled = enabled_ ?? !cam.track.enabled;
			cam.track.enabled = enabled;

			const tr = vc.acquireTransceiver("user", "video");
			if (tr.currentDirection !== "stopped") {
				await tr.sender.replaceTrack(cam.track);
				tr.direction = "sendonly";
			} else {
				voiceLog.warn("camera transceiver is stopped", tr);
			}

			update("camera", enabled);
		},

		async toggleScreenshare() {
			if (store.screensharing) {
				actions.stopScreenshare();
			} else {
				await actions.startScreenshare();
			}
		},

		async startScreenshare() {
			try {
				const screen = await devices.acquireScreenshare();
				const trVideo = vc.acquireTransceiver("screen", "video");
				if (trVideo.currentDirection !== "stopped") {
					await trVideo.sender.replaceTrack(screen.trackVideo);
					trVideo.direction = "sendonly";
				} else {
					voiceLog.warn("screenshare video transceiver is stopped", trVideo);
				}

				if (screen.trackAudio) {
					const trAudio = vc.acquireTransceiver("screen", "audio");
					if (trAudio.currentDirection !== "stopped") {
						await trAudio.sender.replaceTrack(screen.trackAudio);
						trAudio.direction = "sendonly";
					}
				}

				screen.trackVideo.addEventListener("ended", () => {
					actions.stopScreenshare();
				});

				update("screensharing", true);
			} catch (e) {
				handleGetMediaError(e as Error);
			}
		},

		async stopScreenshare() {
			for (const vt of vc.localTransceivers) {
				if (vt.key === "screen") {
					vt.transceiver.sender.replaceTrack(null);
					vt.transceiver.direction = "inactive";
				}
			}
			update("screensharing", false);
		},

		async toggleDeafened(deafened_?: boolean) {
			const deafened = deafened_ ?? !store.deafened;
			update("deafened", deafened);

			// mute all remote audio tracks
			for (const s of vc.streams) {
				for (const track of s.media.getAudioTracks()) {
					// TODO: check if this actually stops rtc from receiving media or if it burns bandwidth
					track.enabled = !deafened;
				}
			}
		},

		async playMusic() {
			try {
				const { track } = await loadMusic();
				// HACK: play music as microphone
				const tr = vc.acquireTransceiver("music", "audio");
				// const tr = vc.acquireTransceiver("user", "audio");
				if (tr.currentDirection !== "stopped") {
					await tr.sender.replaceTrack(track);
					tr.direction = "sendonly";
					update("muted", false);
					update("musicing", true);
				}
			} catch (e) {
				voiceLog.warn("playMusic failed", e);
			}
		},

		// subscribeToParticipant(
		// 	userId: string,
		// 	trackKey: "user" | "screen",
		// 	subscribe: boolean,
		// ) {
		// 	// find the mid for this user's track
		// 	const streamId = `${userId}:${trackKey}`;
		// 	const stream = vc.streams.get(streamId);
		// 	if (!stream) return;

		// 	const subs = stream.mids.map((mid) => ({ mid }));
		// 	if (subscribe) {
		// 		vc.setSubscriptions(subs);
		// 	} else {
		// 		// send empty subscriptions for these mids to unsubscribe
		// 		vc.setSubscriptions([]);
		// 	}
		// },
	};

	return (
		<VoiceContext.Provider value={[store, actions]}>
			{props.children}
		</VoiceContext.Provider>
	);
};

export const useVoice = () => {
	const context = useContext(VoiceContext);
	if (!context) throw new Error("useVoice must be used within a VoiceProvider");
	return context;
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

async function loadMusic() {
	const audio = document.createElement("audio");
	audio.src = "https://dump.celery.eu.org/resoundingly-one-bullsnake.opus";
	audio.crossOrigin = "anonymous";
	await new Promise((res) =>
		audio.addEventListener("loadedmetadata", res, { once: true }),
	);

	const ctx = new AudioContext();
	const source = ctx.createMediaElementSource(audio);
	const dest = ctx.createMediaStreamDestination();
	source.connect(dest);

	await audio.play();

	const track = dest.stream.getAudioTracks()[0];
	return { track, stream: dest.stream };
}
