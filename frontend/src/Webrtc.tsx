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
	conn.addEventListener("icecandidate", (e) => {
		console.info("[rtc:ice] propose candidate", e.candidate);
		if (e.candidate) {
			const c = e.candidate;
			sendWebsocket({
				type: "IceCandidate",
				data: JSON.stringify(c.toJSON()),
			});
		}
	});

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

	let mediaEl!: HTMLAudioElement;

	conn.addEventListener("track", (e) => {
		console.info("[rtc:track] track", e.track, e.streams, e.transceiver);
		mediaEl.srcObject = e.streams[0];
		mediaEl.play();
		mediaEl.addEventListener("canplay", (e) => console.log(e));
		mediaEl.addEventListener("waiting", (e) => console.log(e));
		mediaEl.addEventListener("stalled", (e) => console.log(e));
		mediaEl.addEventListener("change", (e) => console.log(e));
		mediaEl.addEventListener("error", (e) => console.log(e));
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
			} else if (msg.payload.type === "Offer") {
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
		const audio = document.createElement("audio");
		audio.src =
			"https://chat-files.celery.eu.org/media/01969c94-0ac1-7741-a64f-16221a1aa4bf";
		// audio.src = "/zago.opus";
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

	async function playAudioEl_() {
		const video = document.createElement("video");
		video.src = "https://dump.celery.eu.org/hollowly-simple-lorikeet.webm";
		// audio.crossOrigin = "anonymous";
		await new Promise((res) =>
			video.addEventListener("loadedmetadata", res, { once: true })
		);

		const stream: MediaStream = "captureStream" in video
			? (video as any).captureStream()
			: (video as any).mozCaptureStream();
		console.log(video, stream);
		for (const track of stream.getTracks()) {
			const tcr = conn.addTransceiver(track, { streams: [stream] });
			console.log("add transciever", track, tcr);
		}
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
			<audio controls ref={mediaEl}></audio>
		</div>
	);
	// <video controls ref={mediaEl}></video>
};
