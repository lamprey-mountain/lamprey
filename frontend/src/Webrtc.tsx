import { createSignal, For, onCleanup } from "solid-js";
import { useApi } from "./api.tsx";
import { ReactiveMap } from "@solid-primitives/map";
import { createEffect } from "solid-js";

const RTC_CONFIG: RTCConfiguration = {
	iceServers: [
		{ urls: ["stun:relay.webwormhole.io"] },
		{ urls: ["stun:stun.stunprotocol.org"] },
	],
};

export const DebugWebrtc = () => {
	const api = useApi();
	const [conn, setConn] = createSignal<RTCPeerConnection>(
		new RTCPeerConnection(RTC_CONFIG),
	);
	const [rtcState, setRtcState] = createSignal<RTCPeerConnectionState>("new");
	const [voiceState, setVoiceState] = createSignal();

	const tracks = new ReactiveMap<string, MediaStreamTrack>();
	const streams = new ReactiveMap<string, Array<string>>();
	const voiceStates = new ReactiveMap();

	let transceiverMic: RTCRtpTransceiver | undefined;
	let transceiverCam: RTCRtpTransceiver | undefined;
	let transceiverScreenVideo: RTCRtpTransceiver | undefined;
	let transceiverScreenAudio: RTCRtpTransceiver | undefined;
	let reconnectable = true;

	// Helper to get track key from transceiver
	const getTrackKey = (tcr: RTCRtpTransceiver): string => {
		if (tcr === transceiverMic) return "user";
		if (tcr === transceiverCam) return "user";
		if (tcr === transceiverScreenVideo) return "screen";
		if (tcr === transceiverScreenAudio) return "screen";
		throw "unknown track key";
	};

	const getTransceivers = () =>
		[
			transceiverMic,
			transceiverCam,
			transceiverScreenVideo,
			transceiverScreenAudio,
		]
			.filter((tcr): tcr is RTCRtpTransceiver => !!tcr);

	// Complete track metadata using our transceiver references
	const getTrackMetadata = () => {
		return getTransceivers()
			.map((tcr) => ({
				mid: tcr.mid!,
				kind: tcr.sender.track!.kind === "video" ? "Video" : "Audio" as const,
				key: getTrackKey(tcr),
			}));
	};

	setup();

	function setup() {
		// // Reset transceivers on new connection
		// transceiverMic = undefined;
		// transceiverCam = undefined;
		// transceiverScreenVideo = undefined;
		// transceiverScreenAudio = undefined;

		const conn = new RTCPeerConnection(RTC_CONFIG);
		tracks.clear();
		streams.clear();

		conn.addEventListener("connectionstatechange", () => {
			console.warn("[rtc:core] connectionstatechange", conn.connectionState);
			setRtcState(conn.connectionState);

			if (conn.connectionState === "disconnected" && reconnectable) {
				console.log("reconnect");
				setup();
				reconnect();
			}
		});

		console.log(conn);

		conn.addEventListener(
			"icegatheringstatechange",
			() =>
				console.log(
					"[rtc:core] icegatheringstatechange",
					conn.iceGatheringState,
				),
		);

		conn.addEventListener(
			"iceconnectionstatechange",
			() =>
				console.log(
					"[rtc:core] iceconnectionstatechange",
					conn.iceConnectionState,
				),
		);

		// conn.addEventListener("icecandidate", (e) => {
		// 	console.log("[rtc:core] icecandidate", e.candidate);
		// 	sendWebsocket({ type: "Candidate", ...e.candidate?.toJSON() });
		// });

		const negotiate = async () => {
			console.info("[rtc:sdp] create offer");
			const offer = await conn.createOffer();
			await conn.setLocalDescription(offer);
			const tracks = getTrackMetadata();
			console.log("tcrs", tracks);
			sendWebsocket({
				type: "Offer",
				sdp: conn.localDescription!.sdp,
				tracks,
			});
		};

		conn.addEventListener("negotiationneeded", negotiate);

		conn.addEventListener("track", (e) => {
			console.info("[rtc:track] track", e.track, e.streams, e.transceiver);
			tracks.set(e.transceiver.mid!, e.track);
		});

		setRtcState("new");
		setConn(conn);
	}

	api.temp_events.on("sync", async (msg) => {
		const c = conn();
		if (msg.type === "VoiceDispatch") {
			if (msg.payload.type === "Answer") {
				console.log("[rtc:signal] accept answer");
				await c.setRemoteDescription({
					type: "answer",
					sdp: msg.payload.sdp,
				});
			} else if (msg.payload.type === "Offer") {
				if (c.signalingState !== "stable") {
					console.log("[rtc:signal] ignore server offer");
					return;
				}
				console.log("[rtc:signal] accept offer; create answer");
				await c.setRemoteDescription({
					type: "offer",
					sdp: msg.payload.sdp,
				});
				await c.setLocalDescription(await c.createAnswer());
				sendWebsocket({
					type: "Answer",
					sdp: c.localDescription!.sdp,
				});
				// } else if (msg.payload.type === "Candidate") {
				// 	const candidate = JSON.parse(msg.payload.candidate);
				// 	console.log("[rtc:signal] remote ICE candidate", candidate);
				// 	await c.addIceCandidate(candidate);
				// 	// } else if (msg.payload.type === "Subscribe") {
				// 	// 	const { mid } = msg.payload;
				// 	// 	for (const tcr of c.getTransceivers()) {
				// 	// 		console.log(tcr);
				// 	// 		if (tcr.mid === mid) tcr.sender.track!.enabled = true;
				// 	// 	}
			} else if (msg.payload.type === "Have") {
				console.log(
					"[rtc:signal] have from " + msg.payload.user_id,
					msg.payload,
				);
				const { user_id, tracks } = msg.payload;
				for (const track of tracks) {
					const id = `${user_id}:${track.key}`;
					const stream = streams.get(id) ?? [];
					streams.set(id, [...stream, track.mid]);
				}
			} else if (msg.payload.type === "Want") {
				console.log("[rtc:signal] want");
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
		conn().close();
		// // Stop all tracks and clear transceivers
		// [transceiverMic, transceiverCam, transceiverScreenVideo, transceiverScreenAudio].forEach(tcr => {
		//     if (tcr?.sender.track) {
		//         tcr.sender.track.stop();
		//     }
		// });

		// // Reset transceiver references
		// transceiverMic = undefined;
		// transceiverCam = undefined;
		// transceiverScreenVideo = undefined;
		// transceiverScreenAudio = undefined;

		// // Clear other resources
		// tracks.clear();
		// streams.clear();

		// const currentConn = conn();
		// if (currentConn.connectionState !== 'closed') {
		//     currentConn.close();
		// }
	});

	function connect() {
		sendWebsocket({
			type: "VoiceState",
			state: {
				thread_id: "019438f6-bcb4-7d30-ba05-f55cfa4c61d2",
			},
		});
	}

	function connect2() {
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
		conn().close();
		setup();
	}

	function reconnect() {
		console.log("reconnect");
		const c = conn();

		if (transceiverMic?.sender.track) {
			transceiverMic = c.addTransceiver(transceiverMic.sender.track);
		}

		if (transceiverCam?.sender.track) {
			transceiverCam = c.addTransceiver(transceiverCam.sender.track);
		}

		if (transceiverScreenVideo?.sender.track) {
			transceiverScreenVideo = c.addTransceiver(
				transceiverScreenVideo.sender.track,
			);
		}

		if (transceiverScreenAudio?.sender.track) {
			transceiverScreenAudio = c.addTransceiver(
				transceiverScreenAudio.sender.track,
			);
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
		const tcr = conn().addTransceiver(tracks[0]);
		console.log("add transceiver", tcr);
		audio.play();
	}

	const toggleMic = async () => {
		if (transceiverMic?.sender.track) {
			transceiverMic.sender.track.enabled = !transceiverMic.sender.track
				.enabled;
			return;
		}

		const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
		const track = stream.getAudioTracks()[0];
		if (!track) {
			console.warn("no track");
			return;
		}
		const tcr = conn().addTransceiver(track);
		console.log("add transceiver", tcr.mid, tcr);
		track.addEventListener("ended", () => {
			conn().removeTrack(tcr.sender);
		});
		track.enabled = false;
		transceiverMic = tcr;
	};

	const toggleCam = async () => {
		if (transceiverCam?.sender.track) {
			transceiverCam.sender.track.enabled = !transceiverCam.sender.track
				.enabled;
			return;
		}

		const stream = await navigator.mediaDevices.getUserMedia({ video: true });
		const track = stream.getVideoTracks()[0];
		if (!track) {
			console.warn("no track");
			return;
		}
		const tcr = conn().addTransceiver(track);
		console.log("add transceiver", tcr.mid, tcr);
		track.addEventListener("ended", () => {
			conn().removeTrack(tcr.sender);
		});
		// track.enabled = false;
		transceiverCam = tcr;
	};

	const toggleScreen = async () => {
		// TODO
		// if (trackScreenVideo) {
		// 	trackScreenVideo.enabled = !trackScreenVideo.enabled;
		// 	if (trackScreenAudio) {
		// 		trackScreenAudio.enabled = !trackScreenAudio.enabled;
		// 	}
		// 	return;
		// }

		// const stream = await navigator.mediaDevices.getDisplayMedia({
		// 	video: true,
		// 	audio: true,
		// });

		// {
		// 	const track = stream.getVideoTracks()[0];
		// 	if (!track) {
		// 		console.warn("no video track");
		// 		return;
		// 	}
		// 	const tcr = conn().addTransceiver(track);
		// 	console.log("add transceiver", tcr.mid, tcr);
		// 	track.addEventListener("ended", () => {
		// 		conn().removeTrack(tcr.sender);
		// 	});
		// 	trackScreenVideo = track;
		// }

		// {
		// 	const track = stream.getAudioTracks()[0];
		// 	if (!track) {
		// 		console.warn("no audio track");
		// 		return;
		// 	}
		// 	const tcr = conn().addTransceiver(track);
		// 	console.log("add transceiver", tcr.mid, tcr);
		// 	track.addEventListener("ended", () => {
		// 		conn().removeTrack(tcr.sender);
		// 	});
		// 	trackScreenAudio = track;
		// }
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
				{(tcs) => (
					<video
						controls
						autoplay
						ref={(el) => {
							const stream = new MediaStream();
							console.log("add stream", tcs, tracks);
							for (const mid of tcs) {
								const t = tracks.get(mid);
								if (t) {
									console.log("add stream track", mid, t);
									stream.addTrack(t);
								}
							}
							el.srcObject = stream;
						}}
					/>
				)}
			</For>
		</div>
	);
};
