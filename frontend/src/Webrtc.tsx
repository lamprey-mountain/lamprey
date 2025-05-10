import { createSignal, onCleanup } from "solid-js";
import { useApi } from "./api";

const RTC_CONFIG = {
	iceServers: [
		{ urls: ["stun:relay.webwormhole.io"] },
		{ urls: ["stun:stun.stunprotocol.org"] },
		// TODO: selfhost a stun server or two
	],
};

export const DebugWebrtc = () => {
	const api = useApi();
	const conn = new RTCPeerConnection(RTC_CONFIG);
	const [rtcState, setRtcState] = createSignal<RTCPeerConnectionState>("new");

	conn.addEventListener("connectionstatechange", () => {
		console.info("[rtc:core] connectionstatechange", conn.connectionState);
		setRtcState(conn.connectionState);
	});

	// handle interactive connectivity establishment
	// conn.addEventListener("icecandidate", (e) => {
	// 	console.info("[rtc:ice] propose candidate", e.candidate);
	// 	if (e.candidate) {
	// 		const c = e.candidate;
	// 		sendWebsocket({
	// 			type: "IceCandidate",
	// 			candidate: {
	// 				candidate: c.candidate,
	// 				sdpMid: c.sdpMid,
	// 				sdpMLineIndex: c.sdpMLineIndex,
	// 				usernameFragment: c.usernameFragment,
	// 			},
	// 		});
	// 	}
	// });

	conn.addEventListener("icecandidateerror", (e) => {
		console.info("[rtc:ice]", e);
	});

	conn.addEventListener("negotiationneeded", async () => {
		console.info("[rtc:sdp] create offer");
		await negotiate();
	});

	conn.addEventListener("datachannel", (e) => {
		console.info("[rtc:track] datachannel", e.channel);
	});

	conn.addEventListener("track", (e) => {
		console.info("[rtc:track] track", e.track, e.streams, e.transceiver);
		// pc.ontrack = e => audioEl.srcObject = e.streams[0];
	});

	api.temp_events.on("sync", async (msg) => {
		const ws = (api.client as any)._debugGetWebsocket();
		if (msg.type === "VoiceDispatch") {
			console.log("got signalling message", msg.payload);
			if (msg.payload.type === "Answer") {
				console.log("[rtc:signal] accept answer");
				await conn.setRemoteDescription({
					type: "answer",
					sdp: msg.payload.sdp,
				});
			} else {
				console.warn("[rtc:signal] unknown message type");
			}
		}
	});

	onCleanup(() => {
		conn.close();
	});

	console.log(conn);

	async function playAudioEl() {
		// const { audio, stream } = await loadAudioStream();
		const audio = document.createElement("audio");
		audio.src =
			"https://chat-files.celery.eu.org/media/01969c94-0ac1-7741-a64f-16221a1aa4bf";
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
		// tcr.mid
		// tcr.stop
		// conn.rem(tcr);
	}

	async function negotiate() {
		await conn.setLocalDescription(await conn.createOffer());
		sendWebsocket({
			type: "Offer",
			sdp: conn.localDescription!.sdp,
		});
		// const desc = { ...conn.localDescription!.toJSON(), mids: [...media.entries()] };
	}
	globalThis.temp0 = conn;

	async function start() {
		const user_id = api.users.cache.get("@self")!.id;
		console.info("starting with user id " + user_id);
		// await negotiate();
		console.log(conn.createDataChannel("asdf"));
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

	return (
		<div>
			<div>webrtc (nothing to see here, move along...)</div>
			<div>
				<button onClick={start}>start</button>
			</div>
			<div>
				<button onClick={playAudioEl}>play audio</button>
			</div>
			<div>state {rtcState()}</div>
		</div>
	);
};
