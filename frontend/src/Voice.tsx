import { Thread } from "sdk";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import iconCamera from "./assets/camera.png";
import iconHeadphones from "./assets/headphones.png";
import iconMic from "./assets/mic.png";
import iconScreenshare from "./assets/screenshare.png";
import iconSettings from "./assets/settings.png";
import iconX from "./assets/x.png";
import { useApi } from "./api.tsx";
import { ReactiveMap } from "@solid-primitives/map";
import { ToggleIcon } from "./ToggleIcon.tsx";
import { createVoiceClient, SignallingMessage } from "./rtc.ts";

export const Voice = (p: { thread: Thread }) => {
	const api = useApi();
	const rtc = createVoiceClient();
	globalThis.rtc = rtc.conn;

	async function playMusic() {
		const audio = document.createElement("audio");
		audio.src = "https://dump.celery.eu.org/resoundingly-one-bullsnake.opus";
		audio.crossOrigin = "anonymous";
		await new Promise((res) =>
			audio.addEventListener("loadedmetadata", res, { once: true })
		);

		const stream: MediaStream = "captureStream" in audio
			? (audio as any).captureStream()
			: (audio as any).mozCaptureStream();
		const track = stream.getAudioTracks()[0];
		const tcr = rtc.conn.addTransceiver(track);
		console.log("playing music with transceiver", tcr);
		audio.play();
	}

	async function send(payload: SignallingMessage) {
		const ws = api.client.getWebsocket();
		const user_id = api.users.cache.get("@self")!.id;
		console.info("[rtc:signal] send", payload);
		ws.send(JSON.stringify({
			type: "VoiceDispatch",
			user_id,
			payload,
		}));
	}

	let screenTn: RTCRtpTransceiver;

	api.events.on("sync", (e) => {
		const user_id = api.users.cache.get("@self")!.id;
		if (
			e.type === "VoiceState" && e.user_id === user_id && e.state && !screenTn
		) {
			rtc.createStream("screen");
			screenTn = rtc.createTransciever("screen", "video");
		}
	});

	const toggleScreen = async () => {
		const tr = screenTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
			// this version of disabling a track is more aggressive and will re-prompt when toggling
			// tr.stop();
			// await screenTn.sender.replaceTrack(null);
			// screenTn.direction = "inactive";
		} else {
			const stream = await navigator.mediaDevices.getDisplayMedia({
				video: true,
				audio: true,
			}).catch(handleGetMediaError);
			if (!stream) return;

			// TODO: screen sharing audio
			const tr = stream.getVideoTracks()[0];
			if (!tr) {
				console.warn("no video track");
				return;
			}
			await screenTn.sender.replaceTrack(tr);
			screenTn.direction = "sendonly";
			// const user_id = api.users.cache.get("@self")!.id;
			// send({
			// 	type: "Have",
			// 	tracks: [{
			// 		key: "screen",
			// 		kind: "Audio",
			// 		mid: screenTn.mid!,
			// 	}],
			// 	thread_id: p.thread.id,
			// 	user_id,
			// });
		}
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

	return (
		<div>
			<div>see console. rtc state {rtc.state()}</div>
			<div>
				<button onClick={() => rtc.connect(p.thread.id)}>connect</button>
			</div>
			<div>
				<button onClick={rtc.disconnect}>disconnect</button>
			</div>
			<div>
				<button onClick={playMusic}>music</button>
			</div>
			<div>
				<button onClick={toggleScreen}>toggle screen</button>
			</div>
			<For each={rtc.streams()}>
				{(s) => {
					let videoRef!: HTMLVideoElement;
					createEffect(() => {
						console.log("now viewing", s.media);
						if (videoRef) videoRef.srcObject = s.media;
					});
					return (
						<div class="stream">
							stream
							<video
								controls
								autoplay
								playsinline
								ref={videoRef!}
							/>
						</div>
					);
				}}
			</For>
		</div>
	);
};

export const Voice_ = (p: { thread: Thread }) => {
	const rtc = createVoiceClient();

	const [muted, setMuted] = createSignal(false);
	const [deafened, setDeafened] = createSignal(false);

	const api = useApi();
	const [conn, setConn] = createSignal<RTCPeerConnection>(
		new RTCPeerConnection(RTC_CONFIG),
	);
	const [rtcState, setRtcState] = createSignal<RTCPeerConnectionState>("new");
	const [voiceState, setVoiceState] = createSignal();

	const tracks = new ReactiveMap<string, MediaStreamTrack>();
	const streams = new ReactiveMap<string, Array<string>>();
	const voiceStates = new ReactiveMap();

	let transceiverMic: RTCRtpTransceiver;
	let transceiverCam: RTCRtpTransceiver;
	let transceiverScreenVideo: RTCRtpTransceiver;
	let transceiverScreenAudio: RTCRtpTransceiver;
	let transceiverMusic: RTCRtpTransceiver;
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
		const conn = new RTCPeerConnection(RTC_CONFIG);
		tracks.clear();
		streams.clear();

		console.log(conn);

		setRtcState("new");
		setConn(conn);

		transceiverMic = conn.addTransceiver("video", {
			direction: "inactive",
			sendEncodings: [
				// { rid: "q", scaleResolutionDownBy: 4.0, maxBitrate: 150_000 },
				// { rid: "h", scaleResolutionDownBy: 2.0, maxBitrate: 500_000 },
				{ rid: "f" },
			],
		});
		transceiverCam = conn.addTransceiver("audio", { direction: "inactive" });
		transceiverScreenVideo = conn.addTransceiver("video", {
			direction: "inactive",
			sendEncodings: [
				// { rid: "q", scaleResolutionDownBy: 4.0, maxBitrate: 150_000 },
				// { rid: "h", scaleResolutionDownBy: 2.0, maxBitrate: 500_000 },
				{ rid: "f" },
			],
		});
		transceiverScreenAudio = conn.addTransceiver("audio", {
			direction: "inactive",
		});
	}

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
		const track = transceiverMic.sender.track;
		if (track) {
			track.stop();
			await transceiverMic.sender.replaceTrack(null);
			transceiverMic.direction = "inactive";
		} else {
			const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
			const newTrack = stream.getAudioTracks()[0];
			if (!track) {
				console.warn("no track");
				return;
			}

			await transceiverMic.sender.replaceTrack(newTrack);
			transceiverMic.direction = "sendonly";
		}
	};

	const toggleCam = async () => {
		const track = transceiverCam.sender.track;
		if (track) {
			track.stop();
			await transceiverCam.sender.replaceTrack(null);
			transceiverCam.direction = "inactive";
		} else {
			const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
			const newTrack = stream.getAudioTracks()[0];
			if (!track) {
				console.warn("no track");
				return;
			}

			await transceiverCam.sender.replaceTrack(newTrack);
			transceiverCam.direction = "sendonly";
		}
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
						<div class="stream">
							stream
							<video
								controls
								autoplay
								playsinline
								ref={videoRef!}
							/>
						</div>
					);
				}}
			</For>
			<br />
			<br />
			<br />
			<div class="ui">
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
