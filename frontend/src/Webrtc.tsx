import { createSignal, For, onCleanup } from "solid-js";
import { useApi } from "./api.tsx";
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
	let conn = new RTCPeerConnection(RTC_CONFIG);
	const [rtcState, setRtcState] = createSignal<RTCPeerConnectionState>("new");
	const [voiceState, setVoiceState] = createSignal();

	let pendingTracks: RTCRtpTransceiver[] = [];
	const pendingTrackToStream = new Map<string, MediaStream>();

	const tracks = new ReactiveMap<string, MediaStreamTrack>();
	const voiceStates = new ReactiveMap();
	const streams = new ReactiveMap<string, MediaStream>();

	let trackMic: MediaStreamTrack | undefined;
	let trackCam: MediaStreamTrack | undefined;
	let trackScreenVideo: MediaStreamTrack | undefined;
	let trackScreenAudio: MediaStreamTrack | undefined;
	let reconnectable = true;

	const negotiate = async () => {
		console.info("[rtc:sdp] create offer");
		await conn.setLocalDescription(await conn.createOffer());
		sendWebsocket({
			type: "Offer",
			sdp: conn.localDescription!.sdp,
		});
	};

	setup();

	function setup() {
		pendingTracks = [];
		pendingTrackToStream.clear();
		tracks.clear();
		streams.clear();

		conn.addEventListener("connectionstatechange", () => {
			console.info("[rtc:core] connectionstatechange", conn.connectionState);

			if (conn.connectionState === "disconnected" && reconnectable) {
				conn = new RTCPeerConnection(RTC_CONFIG);
				setup();
				reconnect();
			} else {
				setRtcState(conn.connectionState);
			}
		});

		// conn.addEventListener("iceconnectionstatechange", (e) => console.log(e));
		// conn.addEventListener("icecandidate", (e) => console.log(e));
		// conn.addEventListener("icecandidateerror", (e) => console.log(e));
		// conn.addEventListener("icegatheringstatechange", (e) => console.log(e));
		conn.addEventListener("negotiationneeded", negotiate);

		conn.addEventListener("datachannel", (e) => {
			const ch = e.channel;
			console.info("[rtc:track] datachannel", ch);
			// ch.protocol === "Control"
			// ch.protocol === "VoiceActivity"
			// conn.createDataChannel("speaking", {
			// 	protocol: "speaking",
			// });
		});

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
	}

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
					console.log(tcr);
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
				const my_user_id = api.users.cache.get("@self")!.id;
				if (user_id !== my_user_id) {
					// TODO: only subscribe on demand
					sendWebsocket({
						type: "Subscribe",
						mid,
					});
				}
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
		reconnectable = false;
		conn.close();
	});

	function connect() {
		// reset();
		sendWebsocket({
			type: "VoiceState",
			state: {
				thread_id: "019438f6-bcb4-7d30-ba05-f55cfa4c61d2",
			},
		});
	}

	function connect2() {
		// reset();
		sendWebsocket({
			type: "VoiceState",
			state: {
				thread_id: "019761a5-a6fb-70a3-a407-a0d7ffcf2862",
			},
		});
	}

	function reset() {
		sendWebsocket({
			type: "VoiceState",
			state: null,
		});
		conn.close();
	}

	function reconnect() {
		console.log("reconnect");

		if (trackMic) {
			pendingTracks.push(conn.addTransceiver(trackMic));
		}

		if (trackCam) {
			pendingTracks.push(conn.addTransceiver(trackCam));
		}

		if (trackScreenVideo) {
			pendingTracks.push(conn.addTransceiver(trackScreenVideo));
		}

		if (trackScreenAudio) {
			pendingTracks.push(conn.addTransceiver(trackScreenAudio));
		}
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
		console.log("add transceiver", tcr);
		audio.play();
		pendingTracks.push(tcr);
	}

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
		console.log("add transceiver", tcr.mid, tcr);
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
		console.log("add transceiver", tcr.mid, tcr);
		track.addEventListener("ended", () => {
			conn.removeTrack(tcr.sender);
		});
		pendingTracks.push(tcr);
		track.enabled = false;
		trackCam = track;
	};

	const toggleScreen = async () => {
		if (trackScreenVideo) {
			trackScreenVideo.enabled = !trackScreenVideo.enabled;
			if (trackScreenAudio) {
				trackScreenAudio.enabled = !trackScreenAudio.enabled;
			}
			return;
		}

		const stream = await navigator.mediaDevices.getDisplayMedia({
			video: true,
			audio: true,
		});

		{
			const track = stream.getVideoTracks()[0];
			if (!track) {
				console.warn("no video track");
				return;
			}
			const tcr = conn.addTransceiver(track);
			console.log("add transceiver", tcr.mid, tcr);
			track.addEventListener("ended", () => {
				conn.removeTrack(tcr.sender);
			});
			trackScreenVideo = track;
		}

		{
			const track = stream.getAudioTracks()[0];
			if (!track) {
				console.warn("no audio track");
				return;
			}
			const tcr = conn.addTransceiver(track);
			console.log("add transceiver", tcr.mid, tcr);
			track.addEventListener("ended", () => {
				conn.removeTrack(tcr.sender);
			});
			trackScreenAudio = track;
		}
	};

	createEffect(() => {
		console.log("current number of participants:", voiceStates.size);
	});

	createEffect(() => {
		console.log("current number of streams:", streams.size);
	});

	return (
		<div class="webrtc">
			<div>webrtc (nothing to see here, move along...)</div>
			<div>
				<button onClick={connect}>connect</button>
				<button onClick={connect2}>connect2</button>
				<button onClick={reset}>reset/disconnect</button>
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
