import { createSignal, onCleanup, Show } from "solid-js";
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

	let mediaEl!: HTMLAudioElement;

	// const tracks = new Map();
	conn.addEventListener("track", (e) => {
		console.info("[rtc:track] track", e.track, e.streams, e.transceiver);
		// tracks.set(e.transceiver.mid, e.track);

		mediaEl.srcObject = e.streams[0];
		mediaEl.play();
		// mediaEl.addEventListener("canplay", (e) => console.log(e));
		// mediaEl.addEventListener("waiting", (e) => console.log(e));
		// mediaEl.addEventListener("stalled", (e) => console.log(e));
		// mediaEl.addEventListener("change", (e) => console.log(e));
		// mediaEl.addEventListener("error", (e) => console.log(e));
	});

	api.temp_events.on("sync", async (msg) => {
		if (msg.type !== "VoiceDispatch") return;
		console.log("got signalling message", msg.payload);
		if (msg.payload.type === "Answer") {
			console.log("[rtc:signal] accept answer");
			await conn.setRemoteDescription({
				type: "answer",
				sdp: msg.payload.sdp,
			});
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
		} else if (msg.payload.type === "VoiceState") {
			const user_id = api.users.cache.get("@self")!.id;
			if (msg.payload.user_id === user_id) {
				setVoiceState(msg.payload.state);
			}
		} else {
			console.warn("[rtc:signal] unknown message type");
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

	async function connect() {
		disconnect();
		sendWebsocket({
			type: "VoiceState",
			state: {
				thread_id: "fe676818-7b36-429c-98c4-0a8b2fc411b4",
			},
		});

		// const user_id = api.users.cache.get("@self")!.id;
		// console.info("starting with user id " + user_id);
		// console.log(conn.createDataChannel("asdf"));
	}

	async function disconnect() {
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

	const tracks: Record<string, MediaStreamTrack | null> = {
		mic: null,
		cam: null,
		screen: null,
		speaker: null,
	};
	const toggleMic = async () => {
		if (tracks.mic) {
			tracks.mic.enabled = !tracks.mic.enabled;
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
		tracks.mic = track;
	};

	const toggleCam = async () => {
		if (tracks.cam) {
			tracks.cam.enabled = !tracks.cam.enabled;
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
		tracks.cam = track;
	};

	const toggleScreen = async () => {
		if (tracks.display) {
			tracks.display.enabled = !tracks.display.enabled;
			if (tracks.speaker) {
				tracks.speaker.enabled = !tracks.speaker.enabled;
			}
			return;
		}
		const stream = await navigator.mediaDevices.getDisplayMedia({
			video: true,
			audio: true,
		});
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
		tracks.display = track;

		const track2 = stream.getAudioTracks()[0];
		if (!track2) {
			console.warn("no track");
			return;
		}
		const tcr2 = conn.addTransceiver(track2);
		console.log("add transciever", tcr2.mid, tcr2);
		track2.addEventListener("ended", () => {
			conn.removeTrack(tcr2.sender);
		});
		tracks.speaker = track2;
	};

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
			<video controls ref={mediaEl}></video>
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
