import { Room, Thread } from "sdk";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import iconCamera from "./assets/camera.png";
import iconHeadphones from "./assets/headphones.png";
import iconMic from "./assets/mic.png";
import iconScreenshare from "./assets/screenshare.png";
import iconSettings from "./assets/settings.png";
import iconX from "./assets/x.png";
import { useApi } from "./api.tsx";
import { ReactiveMap } from "@solid-primitives/map";

const RTC_CONFIG: RTCConfiguration = {
	iceServers: [
		{ urls: ["stun:relay.webwormhole.io"] },
		{ urls: ["stun:stun.stunprotocol.org"] },
	],
};

export const Voice = (p: { room: Room; thread: Thread }) => {
	const [muted, setMuted] = createSignal(false);
	const [deafened, setDeafened] = createSignal(false);

	// TODO: tooltips
	// const [tip, setTip] = createSignal("some text here")
	// const tooltip = createTooltip({
	// 	tip: () => tip(),
	// });

	const handleMouseOver = (e: MouseEvent) => {
		// const tipEl = ((e.target as HTMLElement).closest("[data-tooltip]") as HTMLElement);
		// if (!tipEl) return;
		// const tipText = tipEl.dataset.tooltip;
		// setTip(tipText as string);
		// tooltip.setContentEl(tipEl)
		// tooltip.showTip();
	};

	const handleMouseOut = (e: MouseEvent) => {
		// const tipEl = ((e.target as HTMLElement).closest("[data-tooltip]") as HTMLElement);
		// if (tipEl) return;
		// tooltip.considerHidingTip()
	};

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
	let transceiverMusic: RTCRtpTransceiver | undefined;
	let reconnectable = true;

	// Helper to get track key from transceiver
	const getTrackKey = (tcr: RTCRtpTransceiver): string => {
		if (tcr === transceiverMic) return "user";
		if (tcr === transceiverCam) return "user";
		if (tcr === transceiverScreenVideo) return "screen";
		if (tcr === transceiverScreenAudio) return "screen";
		if (tcr === transceiverMusic) return "music";
		throw "unknown track key";
	};

	const getTransceivers = () =>
		[
			transceiverMic,
			transceiverCam,
			transceiverScreenVideo,
			transceiverScreenAudio,
			transceiverMusic,
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
				thread_id: p.thread.id,
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
		transceiverMusic = tcr;
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
		const tcr = conn().addTransceiver(track, {
			sendEncodings: [
				// { rid: "q", scaleResolutionDownBy: 4.0, maxBitrate: 150_000 },
				// { rid: "h", scaleResolutionDownBy: 2.0, maxBitrate: 500_000 },
				{ rid: "f" },
			],
		});
		console.log("add transceiver", tcr.mid, tcr);
		track.addEventListener("ended", () => {
			conn().removeTrack(tcr.sender);
		});
		// track.enabled = false;
		transceiverCam = tcr;
	};

	const toggleScreen = async () => {
		if (transceiverScreenVideo) {
			const tv = transceiverScreenVideo.sender.track;
			if (tv) tv.enabled = !tv.enabled;

			if (transceiverScreenAudio) {
				const ta = transceiverScreenAudio.sender.track;
				if (ta) ta.enabled = !ta.enabled;
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
			const tcr = conn().addTransceiver(track, {
				sendEncodings: [
					// { rid: "h", scaleResolutionDownBy: 2.0, maxBitrate: 500_000 },
					{ rid: "f" },
				],
			});
			console.log("add transceiver", tcr.mid, tcr);
			track.addEventListener("ended", () => {
				conn().removeTrack(tcr.sender);
			});
			transceiverScreenVideo = tcr;
		}

		{
			const track = stream.getAudioTracks()[0];
			if (!track) {
				console.warn("no audio track");
				return;
			}
			const tcr = conn().addTransceiver(track);
			console.log("add transceiver", tcr.mid, tcr);
			track.addEventListener("ended", () => {
				conn().removeTrack(tcr.sender);
			});
			transceiverScreenAudio = tcr;
		}
	};

	return (
		<div class="webrtc">
			<div>webrtc (nothing to see here, move along...)</div>
			<div>
				<button onClick={connect}>connect</button>
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
			<div>participants: {voiceStates.size}</div>
			<div>streams: {streams.size}</div>
			<div>
				voice state
				<pre><code>{JSON.stringify(voiceState(), null, 2)}</code></pre>
			</div>
			<For each={[...tracks.entries()]}>
				{([id, track]) => {
					let videoRef!: HTMLVideoElement;
					createEffect(() => {
						if (videoRef) videoRef.srcObject = new MediaStream([track]);
					});
					return (
						<video
							controls
							autoplay
							playsinline
							ref={videoRef!}
						/>
					);
				}}
			</For>
			<br />
			<br />
			<br />
			<div
				class="ui"
				onMouseOver={handleMouseOver}
				onMouseOut={handleMouseOut}
			>
				<div class="row">
					<div style="flex:1">
						<div style="color:green">
							connected
						</div>
						<div>
							room / thread
						</div>
					</div>
					<div>
						<button data-tooltip="arst">
							{/* camera */}
							<img class="icon" src={iconCamera} />
						</button>
						<button>
							{/* camera */}
							<img class="icon" src={iconScreenshare} />
						</button>
						<button>
							{/* disconnect */}
							<img class="icon" src={iconX} />
						</button>
					</div>
				</div>
				<div class="row toolbar">
					<div style="flex:1">user</div>
					<button onClick={() => setMuted((m) => !m)}>
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
	);
};

const ToggleIcon = (props: { checked: boolean; src: string }) => {
	return (
		<svg
			viewBox={`0 0 64 64`}
			role="img"
			class="icon strike"
			aria-checked={props.checked}
		>
			<defs>
				<mask id="strike">
					<rect width="64" height="64" fill="white" />
					<line
						x1="0"
						y1="0"
						x2="64"
						y2="64"
						stroke="black"
						stroke-width="32"
					/>
				</mask>
			</defs>
			<image href={props.src} />
			<line class="line" x1="8" y1="8" x2="56" y2="56" stroke-width="8" />
		</svg>
	);
};
