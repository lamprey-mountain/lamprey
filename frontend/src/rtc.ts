import { createSignal, onCleanup } from "solid-js";
import { useApi } from "./api";
import { SignallingMessage, TrackMetadata } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";

type RemoteStream = {
	id: string;
	user_id: string;
	mids: string[];
	key: string;
	media: MediaStream;
};

type LocalStream = {
	id: string;
	user_id: string;
	transceivers: RTCRtpTransceiver[];
	key: string;
	media: MediaStream;
};

const RTC_CONFIG: RTCConfiguration = {
	iceServers: [
		{ urls: ["stun:relay.webwormhole.io"] },
		{ urls: ["stun:stun.stunprotocol.org"] },
	],
};

export const createVoiceClient = () => {
	const conn = new RTCPeerConnection(RTC_CONFIG);
	const api = useApi();
	const transceivers = new Map<string, RTCRtpTransceiver>();
	const remoteStreams: Array<RemoteStream> = [];
	const localStreams: Array<LocalStream> = [];
	const [rtcState, setRtcState] = createSignal<RTCPeerConnectionState>("new");
	const streams = new ReactiveMap<string, RemoteStream>();

	function setup() {
		conn.addEventListener("connectionstatechange", () => {
			// new, connecting, connected, disconnected, failed, closed
			console.log("[rtc:core] connection state change", conn.connectionState);
			setRtcState(conn.connectionState);
		});

		conn.addEventListener("iceconnectionstatechange", () => {
			// new, checking, connected, completed, failed, disconnected, closed
			console.debug(
				"[rtc:core] ice connection state change",
				conn.iceConnectionState,
			);

			if (conn.iceConnectionState === "failed") {
				console.warn("[rtc:core] connection failed, restarting ice!");
				conn.restartIce();
			}
		});

		conn.addEventListener("signalingstatechange", () => {
			console.debug("[rtc:core] signalling state change", conn.signalingState);
		});

		conn.addEventListener("icegatheringstatechange", () => {
			console.debug(
				"[rtc:core] icegatheringstatechange",
				conn.iceGatheringState,
			);
		});

		conn.addEventListener("icecandidate", (e) => {
			// console.debug("[rtc:core] icecandidate", e.candidate);
			// sendWebsocket({ type: "Candidate", ...e.candidate?.toJSON() });
		});

		conn.addEventListener("negotiationneeded", negotiate);

		conn.addEventListener("track", (e) => {
			const t = e.transceiver;
			console.info("[rtc:track] track", e.track, e.streams, t);
			if (!t.mid) {
				console.warn("transceiver is missing mid");
				return;
			}

			transceivers.set(t.mid, t);

			// add this transceiver to the stream
			const s = remoteStreams.find((s) => s.mids.includes(t.mid!));
			if (s) {
				const tr = t.receiver.track;
				s.media.addTrack(tr);
				console.log("[rtc:stream] added track", tr.kind, "to stream", s.id);
			} else {
				console.warn("[rtc:stream] missing stream, will wait for Have");
			}
		});

		conn.addEventListener("datachannel", (e) => {
			// currently unused
			console.info("[rtc:data] datachannel", e.channel);
		});

		// // TODO: speaking indicators
		// const chanSpeaking = conn.createDataChannel("speaking", {
		// 	ordered: false,
		// 	protocol: "speaking",
		// 	maxRetransmits: 0,
		// });

		// // let people create arbitrary datachannels?
		// const chanStuff = conn.createDataChannel("arbitrary", {
		// 	protocol: "broadcast",
		// });
	}

	function close() {
		conn.close();
		send({ type: "VoiceState", state: null });
	}

	function getTrackMetadata(): TrackMetadata[] {
		const tracks: TrackMetadata[] = [];
		for (const s of localStreams) {
			console.log("[rtc:metadata] local stream %s", s.key);
			for (const t of s.transceivers) {
				if (t.direction === "inactive") {
					console.log("[rtc:metadata] stream is inactive");
					continue;
				}

				const kind = t.sender.track?.kind;
				if (kind) {
					tracks.push({
						key: s.key,
						mid: t.mid!,
						kind: kind === "video" ? "Video" : "Audio",
					});
				} else {
					console.warn("[rtc:metadata] no track for this transceiver");
				}
			}
		}
		return tracks;
	}

	let makingOffer = false;
	let settingRemoteAnswer = false;

	async function negotiate() {
		console.info("[rtc:sdp] negotiation needed");
		try {
			makingOffer = true;
			const offer = await conn.createOffer();
			await conn.setLocalDescription(offer);
			const tracks = getTrackMetadata();
			console.info("[rtc:sdp] create offer", tracks);
			send({
				type: "Offer",
				sdp: conn.localDescription!.sdp,
				tracks,
			});
		} finally {
			makingOffer = false;
		}
	}

	async function send(payload: SignallingMessage) {
		const ws = api.client.getWebsocket();
		const user_id = api.users.cache.get("@self")!.id;
		console.group("[rtc:signal] send", payload.type);
		console.info(payload);
		console.groupEnd();
		ws.send(JSON.stringify({
			type: "VoiceDispatch",
			user_id,
			payload,
		}));
	}

	api.events.on("sync", async (e) => {
		if (e.type === "VoiceState") {
			if (!e.state) {
				console.log("[rtc:stream] clean up tracks from", e.user_id);
				const filtered = remoteStreams.filter((s) => s.user_id !== e.user_id);
				remoteStreams.splice(0, remoteStreams.length, ...filtered);
				for (const [key, s] of streams) {
					if (s.user_id === e.user_id) streams.delete(key);
				}
			}
		} else if (e.type === "VoiceDispatch") {
			if (!api.voiceState()) return;

			const msg = e.payload as SignallingMessage;
			if (msg.type === "Answer") {
				if (conn.signalingState !== "have-local-offer") {
					console.log(
						"[rtc:sdp] ignoring unexpected answer, state:",
						conn.signalingState,
					);
					return;
				}

				console.log("[rtc:sdp] accept answer");
				try {
					settingRemoteAnswer = true;
					await conn.setRemoteDescription({
						type: "answer",
						sdp: msg.sdp,
					});
				} catch (err) {
					console.error("[rtc:sdp] error while accepting answer", err);
					console.log("COPY PASTE THIS", {
						answer: msg.sdp,
						localDescription: conn.localDescription,
					});
				} finally {
					settingRemoteAnswer = false;
				}
			} else if (msg.type === "Offer") {
				const readyForOffer = !makingOffer &&
					(conn.signalingState === "stable" || settingRemoteAnswer);
				if (!readyForOffer) {
					console.log(
						"[rtc:sdp] ignore server offer; signallingState=",
						conn.signalingState,
					);
					return;
				}

				console.log("[rtc:sdp] accept offer; create answer");
				try {
					await conn.setRemoteDescription({
						type: "offer",
						sdp: msg.sdp,
					});
					await conn.setLocalDescription(await conn.createAnswer());
					send({ type: "Answer", sdp: conn.localDescription!.sdp });
				} catch (err) {
					console.error("[rtc:sdp] error while accepting offer", err);
					console.log("COPY PASTE THIS", {
						localDescription: conn.localDescription,
						answer: msg.sdp,
					});
				}

				// // TODO: copy Have logic here?
				// for (const t of msg.tracks) {
				// 	t.kind;
				// 	t.key;
				// 	t.mid;
				// }
			} else if (msg.type === "Candidate") {
				// TODO: handle ice negotiation
				console.log("[rtc:signal] remote ICE candidate");
				// const candidate = JSON.parse(msg.payload.candidate);
				// console.log("[rtc:signal] remote ICE candidate", candidate);
				// await c.addIceCandidate(candidate);
			} else if (msg.type === "Have") {
				const user_id = api.users.cache.get("@self")!.id;
				const ruid = msg.user_id;
				if (ruid === user_id) {
					console.log("[rtc:signal] ignoring Have from self");
					return;
				}

				console.log("[rtc:signal] got Have from %s", ruid, msg.tracks);
				console.log(
					"[rtc:signal] current transceivers",
					conn.getTransceivers().map((t) => [t.mid, t.direction]),
				);
				for (const track of msg.tracks) {
					const streamId = `${ruid}:${track.key}`;
					let s = remoteStreams.find((s) => s.id === streamId);
					if (s) {
						s.mids.push(track.mid);
					} else {
						const media = new MediaStream();
						console.log("[rtc:stream] initialized new stream", streamId, media);
						s = {
							id: streamId,
							user_id: ruid,
							mids: [track.mid],
							key: track.key,
							media,
						};
						remoteStreams.push(s);
						streams.set(streamId, s);
					}

					// create a stream from mids
					for (const mid of s.mids) {
						const tn = transceivers.get(mid);
						if (tn) {
							const tr = tn.receiver.track;
							s.media.addTrack(tr);
							console.log(
								"[rtc:stream] (re)added track",
								tr.kind,
								"to stream",
								streamId,
							);
						} else {
							console.warn(
								"[rtc:stream] missing transceiver, will wait for track event",
							);
						}
					}
				}
			} else if (msg.type === "Want") {
				// TODO: only subscribe to the tracks we want
				// NOTE: `Want` is also called `Subscribe` in some older design notes
				console.log("[rtc:signal] want");
				// const { mid } = msg.payload;
				// for (const tcr of c.getTransceivers()) {
				// 	console.log(tcr);
				// 	if (tcr.mid === mid) tcr.sender.track!.enabled = true;
				// }
			} else {
				console.warn("[rtc:signal] unknown voice dispatch", msg);
			}
		}
	});

	setup();
	onCleanup(close);

	return {
		conn,
		state: rtcState,
		connect(thread_id: string) {
			send({
				type: "VoiceState",
				state: { thread_id },
			});
		},
		disconnect() {
			send({
				type: "VoiceState",
				state: null,
			});
		},
		createStream(key: string) {
			const user_id = api.users.cache.get("@self")!.id;
			console.log("[rtc:local] create local stream", key);
			const media = new MediaStream();
			localStreams.push({
				id: `${user_id}:${key}`,
				user_id,
				transceivers: [],
				key,
				media,
			});
		},
		createTransceiver(
			stream: string,
			kind: "video" | "audio",
			encodings?: RTCRtpEncodingParameters[],
		) {
			const s = localStreams.find((s) => s.key === stream);
			if (!s) throw new Error("could not find that local stream");
			const tr = conn.addTransceiver(kind, {
				direction: "inactive",
				sendEncodings: encodings,
			});
			console.log("[rtc:local] create transceiver", tr);
			s.transceivers.push(tr);
			return tr;
		},
		streams,
	};
};
