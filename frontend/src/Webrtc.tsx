import { createSignal, onCleanup } from "solid-js";
import { useApi } from "./api";
import iconCamera from "./assets/camera.png";
import iconHeadphones from "./assets/headphones.png";
import iconMic from "./assets/mic.png";
import iconScreenshare from "./assets/screenshare.png";
import iconSettings from "./assets/settings.png";
import iconX from "./assets/x.png";
import { createTooltip } from "./Tooltip.tsx";

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
		console.error("[rtc:ice]", e);
	});

	conn.addEventListener("negotiationneeded", async () => {
		console.info("[rtc:sdp] create offer");
		await negotiate();
	});

	conn.addEventListener("datachannel", (e) => {
		const ch = e.channel;
		console.info("[rtc:track] datachannel", ch);
		// ch.protocol === "Control"
		// ch.protocol === "VoiceActivity"
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

	async function negotiate() {
		await conn.setLocalDescription(await conn.createOffer());
		sendWebsocket({
			type: "Offer",
			sdp: conn.localDescription!.sdp,
		});
	}

	onCleanup(() => {
		conn.close();
	});

	console.log(conn);

	async function playAudioEl() {
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
		console.log("add transciever", tcr);
		audio.play();
	}

	async function start() {
		const user_id = api.users.cache.get("@self")!.id;
		console.info("starting with user id " + user_id);
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

	let startedMic = false;
	const toggleMic = async () => {
		if (startedMic) return;
		// await navigator.mediaDevices.getDisplayMedia({ audio: true, video: true })
		// await navigator.mediaDevices.getUserMedia({ video: true })
		// navigator.mediaDevices.enumerateDevices()
		const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
		for (const track of stream.getTracks()) {
			const tcr = conn.addTransceiver(track);
			console.log("add transciever", tcr.mid, tcr);
			track.addEventListener("ended", () => {
				conn.removeTrack(tcr.sender);
			});
		}
		stream.addEventListener("addtrack", (e) => {
			const tcr = conn.addTransceiver(e.track);
			console.log("add transciever", tcr.mid, tcr);
		});
		startedMic = true;
	};

	const toggleCam = async () => {
		// asdf
	};

	const toggleScreen = async () => {
		// asdf
	};

	return (
		<div class="webrtc">
			<div>webrtc (nothing to see here, move along...)</div>
			<div>
				<button onClick={start}>start</button>
			</div>
			<div>
				<button onClick={playAudioEl}>play audio</button>
			</div>
			<div>
				<button onClick={toggleMic}>start mic</button>
				<button onClick={toggleCam}>start cam</button>
				<button onClick={toggleScreen}>start screen</button>
			</div>
			<div>state {rtcState()}</div>
			<audio controls ref={mediaEl}></audio>
			{/* <Ui /> */}
		</div>
	);
};

const Ui = () => {
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

	return (
		<div class="webrtc">
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
