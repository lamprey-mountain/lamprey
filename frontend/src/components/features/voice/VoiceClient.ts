import { ReactiveMap } from "@solid-primitives/map";
import { type Accessor, createSignal, type Setter } from "solid-js";
import type {
	MessageClient,
	SignallingCommand,
	SignallingEvent,
	TrackKey,
	TrackMetadata,
	VoiceState,
	VoiceSubscription,
} from "ts-sdk";
import type { Api } from "@/api";
import { logger } from "@/utils/logger";
import { Speaking } from "./Speaking";
import { RTC_CONFIG } from "./util";

export const log = logger.for("rtc");

/**
 * - pending: needs to connect to sync websocket
 * - connecting:
 * - connected:
 * - disconnected:
 */
export type VoiceConnectionState =
	| "pending"
	| "connecting"
	| "connected"
	| "disconnected";

type LocalStream = {
	id: string;
	user_id: string;
	transceivers: RTCRtpTransceiver[];
	key: string;
	media: MediaStream;
};

type RemoteStream = {
	id: string;
	user_id: string;
	mids: string[];
	key: TrackKey;
	media: MediaStream;
};

/** handles webrtc and signalling */
export class VoiceClient {
	private rtc: RTCPeerConnection | null = null;
	public speaking = new Speaking();
	public connectionState: Accessor<VoiceConnectionState>;
	private setConnectionState: Setter<VoiceConnectionState>;
	public queue: Array<MessageClient> = [];
	private localStreams: Array<LocalStream> = [];
	public streams = new ReactiveMap<string, RemoteStream>();
	private transceivers = new Map<string, RTCRtpTransceiver>();

	private makingOffer = false;
	private settingRemoteAnswer = false;
	private channelId: string | null = null;

	public getRtc() {
		return this.rtc;
	}

	constructor(public api: Api) {
		[this.connectionState, this.setConnectionState] =
			createSignal<VoiceConnectionState>("disconnected");
	}

	/** setup a new rtc peer connection */
	private init(): void {
		if (this.rtc) return;
		this.rtc = new RTCPeerConnection(RTC_CONFIG);
		this.setConnectionState(
			this.api.client.state.get() === "ready" ? "connecting" : "pending",
		);

		// setup event listeners for rtc
		this.rtc.addEventListener("connectionstatechange", () => {
			if (!this.rtc) return;
			log.debug("signal", "connection state change", this.rtc.connectionState);

			const state = this.rtc.connectionState;
			if (state === "connected") {
				this.setConnectionState("connected");
			} else if (state === "failed" || state === "closed") {
				// NOTE: maybe set this to "connecting" too? since VoiceClient should automatically reconnect
				this.setConnectionState("disconnected");
			} else if (state === "connecting" || state === "new") {
				this.setConnectionState("connecting");
			}
		});

		this.rtc.addEventListener("iceconnectionstatechange", () => {
			if (!this.rtc) return;
			log.debug(
				"signal",
				"ice connection state change",
				this.rtc.iceConnectionState,
			);
			if (this.rtc.iceConnectionState === "failed") {
				log.warn("signal", "ice failed, restarting ice", null);
				this.rtc.restartIce();
			}
		});

		this.rtc.addEventListener("signalingstatechange", () => {
			if (!this.rtc) return;
			log.debug("signal", "signaling state change", this.rtc.signalingState);
		});

		this.rtc.addEventListener("icegatheringstatechange", () => {
			if (!this.rtc) return;
			log.debug(
				"signal",
				"ice gathering state change",
				this.rtc.iceGatheringState,
			);
		});

		this.rtc.addEventListener("icecandidate", (e) => {
			if (!this.rtc) return;
			if (!e.candidate?.candidate) return;
			log.debug("signal", "local ice candidate", e.candidate);
			this.sendSignalling({
				type: "Candidate",
				candidate: e.candidate.candidate,
			});
		});

		this.rtc.addEventListener("negotiationneeded", () => {
			if (!this.rtc) return;
			this.negotiate();
		});

		this.rtc.addEventListener("track", (e) => {
			if (!this.rtc) return;
			const t = e.transceiver;
			const track = e.track;
			log.info("rtc", "track", e);
			if (!t.mid) {
				log.warn("rtc", "transceiver missing mid");
				return;
			}

			if (!track) {
				log.warn("rtc", "track event received but track is null");
				return;
			}

			this.transceivers.set(t.mid, t);

			// attach track to a remote stream if we already know the mid
			for (const [, stream] of this.streams) {
				if (stream.mids.includes(t.mid)) {
					stream.media.addTrack(track);
					log.debug(
						"rtc",
						`added track ${t.mid} (${track.kind}) to stream ${stream.id}`,
						stream,
					);
					// trigger reactivity
					this.streams.set(stream.id, { ...stream });
					break;
				}
			}
		});

		this.rtc.addEventListener("datachannel", (e) => {
			if (!this.rtc) return;
			log.debug("rtc", "datachannel", e.channel.label);
		});

		// setup speaking indicator data channel
		const sc = this.rtc.createDataChannel("speaking", {
			ordered: false,
			protocol: "speaking",
			maxRetransmits: 0,
		});

		this.speaking.swapDataChannel(sc);
	}

	public connect(channelId: string): void {
		this.channelId = channelId;
		this.init();

		const existing = this.api.voiceState;
		if (existing) {
			log.warn(
				"signal",
				"already have a voice state, not resetting first",
				existing,
			);
		}

		this.send({
			type: "VoiceConnect",
			voice_state: {
				channel_id: channelId,
				self_mute: true,
				self_deaf: false,
				self_video: false,
				screenshare: null,
			},
		});

		// FIXME: update voice state when local state changes
		// this.send({
		// 	type: "VoiceDispatch",
		// 	channel_id: channelId,
		// 	command: {
		// 		type: "VoiceState",
		// 		state: {
		// 			channel_id: channelId,
		// 			self_mute: true,
		// 			self_deaf: false,
		// 			self_video: false,
		// 			screenshare: null,
		// 		},
		// 	},
		// });
	}

	public disconnect(): void {
		log.info("rtc", "disconnect", null);

		const channelId = this.channelId;
		this.channelId = null;
		this.setConnectionState("disconnected");
		this.rtc?.close();
		this.rtc = null;
		this.transceivers.clear();
		this.localStreams = [];
		this.streams.clear();
		this.queue = [];

		if (!channelId) return;
		this.send({
			type: "VoiceDispatch",
			channel_id: channelId,
			command: {
				type: "Disconnect",
			},
		});
	}

	public send(msg: MessageClient): void {
		this.queue.push(msg);
		this.drainSendQueue();
	}

	/** send a signalling command to the server */
	private sendSignalling(command: SignallingCommand): void {
		if (!this.channelId) {
			log.warn("signal", "no channelId for signalling send", null);
			return;
		}

		this.send({
			type: "VoiceDispatch",
			channel_id: this.channelId,
			command,
		});
	}

	// TEMP(?): public
	public drainSendQueue(): void {
		const currentUser = this.api.users.cache.get("@self");
		const user_id = currentUser?.id;
		if (!user_id) return;
		if (this.connectionState() === "pending") return;

		for (const msg of this.queue) {
			log.info("signal", "send " + msg.type, msg);
			this.api.client.send(msg);
		}

		this.queue.splice(0, this.queue.length);
	}

	private async negotiate(): Promise<void> {
		if (!this.rtc) return;
		log.info("signal", "negotiation needed", "");
		try {
			this.makingOffer = true;
			const offer = await this.rtc.createOffer();
			await this.rtc.setLocalDescription(offer);
			const tracks = this.getTrackMetadata();
			log.info("signal", "create offer", tracks);
			this.sendSignalling({
				type: "Offer",
				sdp: this.rtc.localDescription?.sdp ?? "",
				tracks,
			});
		} finally {
			this.makingOffer = false;
		}
	}

	private getTrackMetadata(): TrackMetadata[] {
		const tracks: TrackMetadata[] = [];
		for (const s of this.localStreams) {
			for (const t of s.transceivers) {
				if (t.direction === "inactive") continue;
				const kind = t.sender.track?.kind;
				if (kind) {
					tracks.push({
						key: s.key as "user" | "screen",
						mid: t.mid!,
						kind: kind as "video" | "audio",
					});
				}
			}
		}
		return tracks;
	}

	public async handleVoiceState(
		uid: string,
		vs: VoiceState | null,
	): Promise<void> {
		if (!vs) {
			log.debug("signal", "clean up tracks from " + uid, null);
			// remove all streams belonging to this user
			for (const [key, s] of this.streams) {
				if (s.user_id === uid) this.streams.delete(key);
			}
		}
	}

	public async handleSignalingEvent(msg: SignallingEvent): Promise<void> {
		if (!this.rtc) return;

		switch (msg.type) {
			case "Connected": {
				this.setConnectionState("connected");
				this.drainSendQueue();
				// send initial Want subscriptions for all streams we know about
				break;
			}

			case "Offer": {
				const readyForOffer =
					!this.makingOffer &&
					(this.rtc.signalingState === "stable" || this.settingRemoteAnswer);
				if (!readyForOffer) {
					log.debug(
						"signal",
						"ignore server offer; signalingState=",
						this.rtc.signalingState,
					);
					return;
				}

				log.debug("signal", "accept offer; create answer");
				try {
					await this.rtc.setRemoteDescription({
						type: "offer",
						sdp: msg.sdp,
					});
					await this.rtc.setLocalDescription(await this.rtc.createAnswer());
					this.sendSignalling({
						type: "Answer",
						sdp: this.rtc.localDescription?.sdp ?? "",
					});
				} catch (err) {
					log.error("signal", "error while accepting offer", err);
				}

				// process track metadata from the offer
				for (const track of msg.tracks) {
					this.processRemoteTrack(track, track.user_id);
				}

				break;
			}

			case "Answer": {
				if (this.rtc.signalingState !== "have-local-offer") {
					log.debug(
						"signal",
						"ignoring unexpected answer, state:",
						this.rtc.signalingState,
					);
					return;
				}

				log.debug("signal", "accept answer");
				try {
					this.settingRemoteAnswer = true;
					await this.rtc.setRemoteDescription({ type: "answer", sdp: msg.sdp });
				} catch (err) {
					log.error("signal", "error while accepting answer", err);
				} finally {
					this.settingRemoteAnswer = false;
				}
				break;
			}

			case "Candidate": {
				log.debug("signal", "remote ice candidate", msg.candidate);
				// TODO: pass sdpMid/sdpMLineIndex/usernameFragment to addIceCandidate
				await this.rtc.addIceCandidate({ candidate: msg.candidate });
				break;
			}

			case "Migrate": {
				// TODO: handle
				// create a new RTCPeerConnection, but keep the old one until the new conn is ready to use
				break;
			}

			case "Disconnected": {
				// TODO: handle
				// shut down rtc connection. maybe set state to errored/failed if the disconnection wasnt intentional?
				break;
			}

			case "Tracks": {
				const selfUser = this.api.users.cache.get("@self");
				if (msg.user_id === selfUser?.id) {
					log.debug("signal", "ignoring Tracks from self", msg.tracks);
					return;
				}

				log.debug("signal", "got Tracks from " + msg.user_id, msg.tracks);
				for (const track of msg.tracks) {
					this.processRemoteTrack(track, msg.user_id);
				}
				break;
			}

			case "Subscribe": {
				log.debug("signal", "got Subscribe", msg.subs);
				// TODO: handle server-sent subscriptions
				break;
			}

			case "Error": {
				log.error("signal", "got SignallingEvent::Error", msg);
				break;
			}

			default: {
				log.warn("signal", "unknown voice dispatch", msg);
			}
		}
	}

	/** process a TrackMetadata entry from Have/Offer, building RemoteStream entries */
	private processRemoteTrack(track: TrackMetadata, uid: string): void {
		// user_id may come from the Have message; for Offer, tracks belong to the server-indicated peer
		const streamId = `${uid}:${track.key}`;

		let s = this.streams.get(streamId);
		if (s) {
			log.debug("rtc", `reuse stream ${streamId}`, s);
			if (!s.mids.includes(track.mid)) s.mids.push(track.mid);
		} else {
			const media = new MediaStream();
			log.debug("rtc", `initialized new stream ${streamId}`, media);
			s = {
				id: streamId,
				user_id: uid,
				mids: [track.mid],
				key: track.key,
				media,
			};
			this.streams.set(streamId, s);
		}

		// attach already-arrived transceiver tracks
		for (const mid of s.mids) {
			const tn = this.transceivers.get(mid);
			if (tn) {
				s.media.addTrack(tn.receiver.track);
				log.debug("rtc", `(re)added track ${mid} to stream ${streamId}`, null);
			}
		}

		// trigger reactivity
		this.streams.set(streamId, { ...s });
	}

	public setSubscriptions(subs: Array<VoiceSubscription>): void {
		this.sendSignalling({ type: "Subscribe", subs });
	}

	public acquireTransceiver(
		key: string,
		kind: "audio" | "video",
		encodings?: RTCRtpEncodingParameters[],
	): RTCRtpTransceiver {
		if (!this.rtc) throw new Error("RTCPeerConnection not initialized");

		// reuse if we already have a transceiver for this key + kind
		const stream = this.localStreams.find((s) => s.key === key);
		if (stream) {
			const existing = stream.transceivers.find(
				(t) => t.sender.track?.kind === kind,
			);
			if (existing) return existing;
		}

		const tr = this.rtc.addTransceiver(kind, {
			direction: "inactive",
			sendEncodings: encodings,
		});
		log.info("rtc", "create transceiver", tr);

		// attach to local stream
		const ls = this.getLocalStream(key);
		ls.transceivers.push(tr);

		return tr;
	}

	public getLocalStream(key: string) {
		const currentUser = this.api.users.cache.get("@self");
		const user_id = currentUser?.id;
		if (!user_id) throw "a"; // TODO: better errors

		const existing = this.localStreams.find(
			(i) => i.key === key && i.user_id === user_id,
		);
		if (existing) {
			log.debug("rtc", "reuse local stream " + key, existing);
			return existing;
		}
		log.debug("rtc", "create local stream", key);
		const media = new MediaStream();
		const s: LocalStream = {
			id: `${user_id}:${key}`,
			user_id,
			transceivers: [],
			key,
			media,
		};
		this.localStreams.push(s);
		return s;
	}

	private migrate() {
		// TODO: how will this work?
		// 1. create new rtc instance
		// 2. recreate existing transceivers on new rtc instance
		// 3. close old rtc instance
	}
}
