import { createSignal, For, onCleanup, Show } from "solid-js";
import { useApi } from "./api.tsx";
import iconCamera from "./assets/camera.png";
import iconHeadphones from "./assets/headphones.png";
import iconMic from "./assets/mic.png";
import iconScreenshare from "./assets/screenshare.png";
import iconSettings from "./assets/settings.png";
import iconX from "./assets/x.png";
import { ReactiveMap } from "@solid-primitives/map";
import { createEffect } from "solid-js";

const RTC_CONFIG = {
	iceServers: [
		{ urls: ["stun:relay.webwormhole.io"] },
		{ urls: ["stun:stun.stunprotocol.org"] },
	],
};

export const DebugWebrtc = () => {
	const api = useApi();
	const conn = new RTCPeerConnection(RTC_CONFIG);
	const [rtcState, setRtcState] = createSignal<RTCPeerConnectionState>("new");
	const [voiceState, setVoiceState] = createSignal();

	conn.addEventListener("connectionstatechange", () => {
		console.info("[rtc:core] connectionstatechange", conn.connectionState);
		setRtcState(conn.connectionState);
	});

	let pendingTracks: RTCRtpTransceiver[] = [];
	let pendingTrackToStream = new Map<string, MediaStream>();

	conn.addEventListener("negotiationneeded", async () => {
		console.info("[rtc:sdp] create offer");
		await conn.setLocalDescription(await conn.createOffer());
		sendWebsocket({
			type: "Offer",
			sdp: conn.localDescription!.sdp,
		});
	});

	conn.addEventListener("datachannel", (e) => {
		const ch = e.channel;
		console.info("[rtc:track] datachannel", ch);
		// ch.protocol === "Control"
		// ch.protocol === "VoiceActivity"
	});

	const tracks = new ReactiveMap();
	const voiceStates = new ReactiveMap();
	const streams = new ReactiveMap<string, MediaStream>();

	let trackMic: MediaStreamTrack | undefined;
	let trackCam: MediaStreamTrack | undefined;

	conn.addEventListener("track", (e) => {
		console.info("[rtc:track] track", e.track, e.streams, e.transceiver);
		// tracks.set(e.transceiver.mid, e.transceiver);
		tracks.set(e.transceiver.mid, e.track);
		const s = pendingTrackToStream.get(e.transceiver.mid!);
		if (s) {
			s.addTrack(e.track);
			pendingTrackToStream.delete(e.transceiver.mid!);
		}
	});

	api.temp_events.on("sync", async (msg) => {
		if (msg.type === "VoiceDispatch") {
			console.log("got signalling message", msg.payload);
			if (msg.payload.type === "Answer") {
				console.log("[rtc:signal] accept answer");
				await conn.setRemoteDescription({
					type: "answer",
					sdp: msg.payload.sdp,
				});
				console.log({ pendingTracks });
				for (const tcr of pendingTracks) {
					sendWebsocket({
						type: "Publish",
						mid: tcr.mid,
						kind: tcr.sender.track?.kind === "video" ? "Video" : "Audio",
						key: "user",
					});
				}
				pendingTracks = [];
			} else if (msg.payload.type === "Offer") {
				if (conn.signalingState !== "stable") {
					console.log("[rtc:signal] ignore server offer");
					return;
				}
				console.log("[rtc:signal] accept offer; create answer");
				await conn.setRemoteDescription({
					type: "offer",
					sdp: msg.payload.sdp,
				});
				await conn.setLocalDescription(await conn.createAnswer());
				sendWebsocket({
					type: "Answer",
					sdp: conn.localDescription!.sdp,
				});
			} else if (msg.payload.type === "Subscribe") {
				const { mid } = msg.payload;
				for (const tcr of conn.getTransceivers()) {
					if (tcr.mid === mid) tcr.sender.track!.enabled = true;
				}
			} else if (msg.payload.type === "Publish") {
				const { user_id, key, mid } = msg.payload;
				const stream = streams.get(`${user_id}:${key}`) ?? new MediaStream();
				const t = tracks.get(mid);
				if (t) {
					stream.addTrack(t);
				} else {
					pendingTrackToStream.set(mid, stream);
				}
				streams.set(`${user_id}:${key}`, stream);
			} else {
				console.warn("[rtc:signal] unknown message type");
			}
		} else if (msg.type === "VoiceState") {
			const user_id = api.users.cache.get("@self")!.id;
			if (msg.user_id === user_id) {
				setVoiceState(msg.state);
			}
			if (msg.state) {
				voiceStates.set(msg.user_id, msg.state);
			} else {
				voiceStates.delete(msg.user_id);
			}
			console.log(
				"[voice:state] update voice state for %s",
				msg.user_id,
				msg.state,
			);
		}
	});

	onCleanup(() => {
		disconnect();
		conn.close();
	});

	console.log(conn);

	async function playAudioEl() {
		const audio = document.createElement("audio");
		// audio.src =
		// 	"https://chat-files.celery.eu.org/media/01969c94-0ac1-7741-a64f-16221a1aa4bf";
		audio.src = "https://dump.celery.eu.org/resoundingly-one-bullsnake.opus";
		audio.crossOrigin = "anonymous";
		await new Promise((res) =>
			audio.addEventListener("loadedmetadata", res, { once: true })
		);

		const stream: MediaStream = "captureStream" in audio
			? (audio as any).captureStream()
			: (audio as any).mozCaptureStream();
		const tracks = stream.getAudioTracks();
		console.log(audio, stream, tracks);
		if (tracks.length > 1) {
			console.warn("audio has multiple tracks, using first one", tracks);
		}
		const tcr = conn.addTransceiver(tracks[0]);
		console.log("add transciever", tcr);
		audio.play();
	}

	function connect() {
		disconnect();
		sendWebsocket({
			type: "VoiceState",
			state: {
				thread_id: "019761a5-a6fb-70a3-a407-a0d7ffcf2862",
			},
		});
	}

	function disconnect() {
		sendWebsocket({
			type: "VoiceState",
			state: null,
		});
	}

	function sendWebsocket(payload: any) {
		const ws = (api.client as any)._debugGetWebsocket() as WebSocket;
		const user_id = api.users.cache.get("@self")!.id;
		console.info("send websocket dispatch", payload);
		ws.send(JSON.stringify({
			type: "VoiceDispatch",
			user_id,
			payload,
		}));
	}
	console.log(sendWebsocket);

	const toggleMic = async () => {
		if (trackMic) {
			trackMic.enabled = !trackMic.enabled;
			return;
		}

		const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
		const track = stream.getAudioTracks()[0];
		if (!track) {
			console.warn("no track");
			return;
		}
		const tcr = conn.addTransceiver(track);
		console.log("add transciever", tcr.mid, tcr);
		track.addEventListener("ended", () => {
			conn.removeTrack(tcr.sender);
		});
		track.enabled = false;
		pendingTracks.push(tcr);
		trackMic = track;
	};

	const toggleCam = async () => {
		if (trackCam) {
			trackCam.enabled = !trackCam.enabled;
			return;
		}

		const stream = await navigator.mediaDevices.getUserMedia({ video: true });
		const track = stream.getVideoTracks()[0];
		if (!track) {
			console.warn("no track");
			return;
		}
		const tcr = conn.addTransceiver(track);
		console.log("add transciever", tcr.mid, tcr);
		track.addEventListener("ended", () => {
			conn.removeTrack(tcr.sender);
		});
		pendingTracks.push(tcr);
		track.enabled = false;
		trackCam = track;
	};

	const toggleScreen = async () => {
		// if (tracks.display) {
		// 	tracks.display.enabled = !tracks.display.enabled;
		// 	if (tracks.speaker) {
		// 		tracks.speaker.enabled = !tracks.speaker.enabled;
		// 	}
		// 	return;
		// }
		// const stream = await navigator.mediaDevices.getDisplayMedia({
		// 	video: true,
		// 	audio: true,
		// });
		// const track = stream.getVideoTracks()[0];
		// if (!track) {
		// 	console.warn("no track");
		// 	return;
		// }
		// const tcr = conn.addTransceiver(track);
		// console.log("add transciever", tcr.mid, tcr);
		// track.addEventListener("ended", () => {
		// 	conn.removeTrack(tcr.sender);
		// });
		// tracks.display = track;

		// const track2 = stream.getAudioTracks()[0];
		// if (!track2) {
		// 	console.warn("no track");
		// 	return;
		// }
		// const tcr2 = conn.addTransceiver(track2);
		// console.log("add transciever", tcr2.mid, tcr2);
		// track2.addEventListener("ended", () => {
		// 	conn.removeTrack(tcr2.sender);
		// });
		// tracks.speaker = track2;
	};

	createEffect(() => {
		console.log("current number of participants:", voiceStates.size);
	});

	return (
		<div class="webrtc">
			<div>webrtc (nothing to see here, move along...)</div>
			<div>
				<Show
					when={voiceState()}
					fallback={<button onClick={connect}>connect</button>}
				>
					<button onClick={disconnect}>disconnect</button>
				</Show>
			</div>
			<div>
				<button onClick={playAudioEl}>play audio</button>
			</div>
			<div>
				<button onClick={toggleMic}>start mic</button>
				<button onClick={toggleCam}>start cam</button>
				<button onClick={toggleScreen}>start screen</button>
			</div>
			<div>rtc state {rtcState()}</div>
			<div>
				voice state
				<pre><code>{JSON.stringify(voiceState(), null, 2)}</code></pre>
			</div>
			<For each={[...streams.values()]}>
				{(t) => (
					<video
						controls
						autoplay
						ref={(el) => el.srcObject = t}
					/>
				)}
			</For>
		</div>
	);
};

type VoiceState = any;

// per user
type Participant = {
	state: VoiceState;

	tracks: Record<"mic" | "cam" | "screen", {
		mid: string;
		kind: "video" | "audio";
		rids: number[];
		enabled: boolean;
	}>;
};

type TrackState = "pending" | "negotiating" | "open";
