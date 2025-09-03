import { Thread } from "sdk";
import { createEffect, createSignal, For } from "solid-js";
import iconCamera from "./assets/camera.png";
import iconHeadphones from "./assets/headphones.png";
import iconMic from "./assets/mic.png";
import iconScreenshare from "./assets/screenshare.png";
import iconSettings from "./assets/settings.png";
import iconX from "./assets/x.png";
import { useApi } from "./api.tsx";
import { ReactiveMap } from "@solid-primitives/map";
import { ToggleIcon } from "./ToggleIcon.tsx";
import { createVoiceClient, VoiceState } from "./rtc.ts";

export const Voice = (p: { thread: Thread }) => {
	const api = useApi();
	const rtc = createVoiceClient();

	// TEMP: debugging
	(globalThis as any).rtc = rtc.conn;

	let screenVidTn: RTCRtpTransceiver;
	let screenAudTn: RTCRtpTransceiver;
	let micTn: RTCRtpTransceiver;
	let camTn: RTCRtpTransceiver;
	let musicTn: RTCRtpTransceiver;

	// TODO: move voice state stuff to main api
	const [voiceState, setVoiceState] = createSignal();
	const voiceStates = new ReactiveMap();

	api.events.on("sync", (e) => {
		const user_id = api.users.cache.get("@self")!.id;
		if (e.type === "VoiceState") {
			const state = e.state as VoiceState | null;
			console.log("[rtc:signal] recv voice state", state);
			if (state) {
				voiceStates.set(e.user_id, state);
			} else {
				voiceStates.delete(e.user_id);
			}
			if (e.user_id === user_id) {
				setVoiceState(state);
			}
		}
		if (
			e.type === "VoiceState" && e.user_id === user_id && e.state &&
			!screenVidTn
		) {
			rtc.createStream("user");
			rtc.createStream("screen");
			rtc.createStream("music");
			micTn = rtc.createTransceiver("user", "audio");
			camTn = rtc.createTransceiver("user", "video");
			screenAudTn = rtc.createTransceiver("screen", "audio");
			screenVidTn = rtc.createTransceiver("screen", "video");
			musicTn = rtc.createTransceiver("music", "audio");
		}
	});

	const playMusic = async () => {
		const tr = musicTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
		} else {
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
			await musicTn.sender.replaceTrack(track);
			musicTn.direction = "sendonly";
			console.log("playing music with transceiver", musicTn);
			audio.play();
		}
	};

	const toggleScreen = async () => {
		const tr = screenVidTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
			const t = screenAudTn.sender.track;
			if (t) t.enabled = !tr.enabled;
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

			{
				const tr = stream.getAudioTracks()[0];
				if (!tr) {
					console.warn("no audio track");
					return;
				}
				await screenAudTn.sender.replaceTrack(tr);
				screenAudTn.direction = "sendonly";
			}

			{
				const tr = stream.getVideoTracks()[0];
				if (!tr) {
					console.warn("no video track");
					return;
				}
				await screenVidTn.sender.replaceTrack(tr);
				screenVidTn.direction = "sendonly";
			}

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

	const toggleMic = async () => {
		const tr = micTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
		} else {
			const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
				.catch(handleGetMediaError);
			if (!stream) return;

			const track = stream.getAudioTracks()[0];
			if (!track) {
				console.warn("no track");
				return;
			}

			await micTn.sender.replaceTrack(track);
			micTn.direction = "sendonly";
		}
	};

	const toggleCam = async () => {
		const tr = camTn.sender.track;
		if (tr) {
			tr.enabled = !tr.enabled;
		} else {
			const stream = await navigator.mediaDevices.getUserMedia({ video: true })
				.catch(handleGetMediaError);
			if (!stream) return;

			const track = stream.getVideoTracks()[0];
			if (!track) {
				console.warn("no track");
				return;
			}

			await camTn.sender.replaceTrack(track);
			camTn.direction = "sendonly";
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

	const [muted, setMuted] = createSignal(false);
	const [deafened, setDeafened] = createSignal(false);

	return (
		<div class="webrtc">
			<div>coming soon... see console for debug. rtc state: {rtc.state()}</div>
			<div>
				<button onClick={() => rtc.connect(p.thread.id)}>connect</button>
				<button onClick={rtc.disconnect}>disconnect</button>
			</div>
			<div>
				<button onClick={playMusic}>music</button>
			</div>
			<div>
				<button onClick={toggleCam}>toggle cam</button>
				<button onClick={toggleMic}>toggle mic</button>
				<button onClick={toggleScreen}>toggle screen</button>
			</div>
			<div>participants: {voiceStates.size}</div>
			<div>streams: {rtc.streams().length}</div>
			<div>
				voice state
				<pre><code>{JSON.stringify(voiceState(), null, 2)}</code></pre>
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
						<button>
							{/* disconnect */}
							<img class="icon" src={iconX} />
						</button>
					</div>
				</div>
				<div class="row">
					<div style="flex:1"></div>
					<div>
						<button data-tooltip="arst">
							{/* camera */}
							<img class="icon" src={iconCamera} />
						</button>
						<button>
							{/* camera */}
							<img class="icon" src={iconScreenshare} />
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
