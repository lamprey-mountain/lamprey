import { ReactiveMap } from "@solid-primitives/map";
import { type Accessor, createSignal, type Setter } from "solid-js";
import type {
	MediaKind,
	MessageClient,
	SignallingCommand,
	SignallingEvent,
	TrackCreate,
	TrackKey,
	TrackMetadata,
	VoiceState,
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

export type VoiceTransceiver = {
	key: TrackKey;
	transceiver: RTCRtpTransceiver;
};

export type VoiceStream = {
	/** unique stream id formatted as `${user_id}:${key}` */
	id: string;

	/** user whos publishing this stream */
	user_id: string;

	/** ids of tracks in this stream, including ones that we aren't subscribed to */
	track_ids: string[]; // TODO: make this a Set

	key: TrackKey;
	media: MediaStream;
};

export type VoiceTrack = {
	user_id: string;
	metadata: TrackMetadata;

	/**
	 * the id of this track
	 *
	 * may be undefined if this track only exists locally.
	 */
	id?: string;

	/**
	 * the local mid of this track
	 *
	 * may be undefined if this track isn't subscribed to.
	 */
	mid?: string;
};

/** handles webrtc and signalling */
export class VoiceClient {
	private rtc: RTCPeerConnection | null = null;
	public speaking = new Speaking();
	public connectionState: Accessor<VoiceConnectionState>;
	private setConnectionState: Setter<VoiceConnectionState>;
	public queue: Array<MessageClient> = [];

	/** mapping of mid -> voice transceiver*/
	private transceivers = new Map<string, VoiceTransceiver>();

	/** array of local transceivers */
	public localTransceivers: VoiceTransceiver[] = [];

	/** mapping of track id -> track */
	public tracks = new ReactiveMap<string, VoiceTrack>();

	/** mapping of stream id -> stream */
	public streams = new ReactiveMap<string, VoiceStream>();

	/** set of subscribed track ids */
	public subscribedTracks = new Set<string>();

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

			// this isn't really used much, maybe i should remove it...
			// im unsure if ice lite means that i can skip this or not?
			// log.debug("signal", "local ice candidate", e.candidate);
			// this.sendSignalling({
			// 	type: "Candidate",
			// 	candidate: e.candidate.candidate,
			// });
		});

		this.rtc.addEventListener("negotiationneeded", () => {
			if (!this.rtc) return;
			this.negotiate();
		});

		this.rtc.addEventListener("track", (e) => {
			if (!this.rtc) return;
			const t = e.transceiver;
			const rtcTrack = e.track;
			log.info("rtc", "track", e);
			if (!t.mid) {
				log.warn("rtc", "transceiver missing mid");
				return;
			}

			if (!rtcTrack) {
				log.warn("rtc", "track event received but track is null");
				return;
			}

			// lookup the mid this track is associated with
			const track = [...this.tracks.values()].find((x) => x.mid === t.mid);
			if (!track) {
				// the server always sends track mapping in Offer, so we should never receive tracks with mids that aren't registered
				log.warn("rtc", "received track for unknown mid", t.mid);
				return;
			}

			if (track.id) {
				const stream = this.getStream(track.user_id, track.metadata.key);
				if (!stream.track_ids.includes(track.id)) {
					stream.track_ids.push(track.id);
				}

				stream.media.addTrack(rtcTrack);
				log.debug(
					"rtc",
					`added track ${t.mid} (${rtcTrack.kind}) to stream ${stream.user_id}:${stream.key}`,
					stream,
				);

				this.transceivers.set(t.mid, {
					key: track.metadata.key,
					transceiver: t,
				});

				// force trigger reactivity in solid ReactiveMap
				// NOTE: this may cause a flash as the stream updates, i should find some way to prevent this
				this.streams.delete(stream.id);
				this.streams.set(stream.id, stream);
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

		// FIXME: also update voice state when local state changes
		// this probably should be done outside of VoiceClient? ie. context reactively calls client.updateVoiceState or something?
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
		this.localTransceivers = [];
		this.tracks.clear();
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
			log.info(
				"signal",
				`send ${msg.type}${msg.type === "VoiceDispatch" ? ` (${msg.command.type})` : ""}`,
				msg,
			);
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

	private getTrackMetadata(): TrackCreate[] {
		const tracks: TrackCreate[] = [];

		for (const vt of this.localTransceivers) {
			if (vt.transceiver.direction === "inactive") continue;

			if (vt.transceiver.mid) {
				this.transceivers.set(vt.transceiver.mid, vt);
				const kind = vt.transceiver.sender.track?.kind;
				if (!kind) continue;
				tracks.push({
					key: vt.key,
					mid: vt.transceiver.mid,
					kind: kind as MediaKind,
				});
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
			for (const stream of Array.from(this.streams.values())) {
				if (stream.user_id === uid) {
					this.streams.delete(stream.id);
				}
			}
			// remove all tracks belonging to this user
			for (const [trackId, track] of Array.from(this.tracks.entries())) {
				if (track.user_id === uid) {
					this.tracks.delete(trackId);
				}
			}
		}
	}

	public async handleSignalingEvent(msg: SignallingEvent): Promise<void> {
		if (!this.rtc) return;

		switch (msg.type) {
			case "Connected": {
				this.setConnectionState("connected");
				this.drainSendQueue();
				// TODO: send initial subscriptions
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

				// process track mapping from the offer
				// NOTE: how do i handle mappings when ignoring server offer?
				for (const mapping of msg.tracks) {
					const track = this.tracks.get(mapping.id);
					if (track) {
						track.mid = mapping.mid;
						this.tracks.set(mapping.id, { ...track, mid: mapping.mid });
					} else {
						log.warn(
							"signal",
							"offer mapping for unknown track id",
							mapping.id,
						);
					}
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
				// shut down rtc connection. maybe set state to errored/failed if the disconnection wasnt intentional? maybe try to reconnect?
				break;
			}

			case "Tracks": {
				const selfUser = this.api.users.cache.get("@self");
				if (msg.user_id === selfUser?.id) {
					log.debug("signal", "ignoring Tracks from self", msg);
					return;
				}

				log.debug("signal", "got Tracks from " + msg.user_id);
				if (msg.added) {
					for (const announcement of msg.added) {
						const trackId = announcement.id;
						this.tracks.set(trackId, {
							user_id: msg.user_id,
							metadata: announcement,
							id: trackId,
						});

						const stream = this.getStream(msg.user_id, announcement.key);
						if (!stream.track_ids.includes(trackId)) {
							stream.track_ids.push(trackId);
						}
					}
				}

				if (msg.removed) {
					for (const trackId of msg.removed) {
						const track = this.tracks.get(trackId);
						if (track) {
							const stream = this.getStream(track.user_id, track.metadata.key);
							stream.track_ids = stream.track_ids.filter(
								(id) => id !== trackId,
							);
							// FIXME: remove track
							// this.transceivers.get(...)?.transceiver.receiver.track;
							// stream.media.removeTrack();
							this.tracks.delete(trackId);
						}
					}
				}

				break;
			}

			case "Subscribe": {
				log.debug("signal", "got Subscribe", msg);
				// TODO: handle server-sent subscriptions
				// TODO: advertise tracks but dont send them unless the sfu requests it (this requires updating the signalling protocol)
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

	/**
	 * Get or create a stream for a given user and key
	 */
	public getStream(user_id: string, key: TrackKey): VoiceStream {
		const streamId = `${user_id}:${key}`;
		const existing = this.streams.get(streamId);
		if (existing) return existing;

		const media = new MediaStream();
		const stream: VoiceStream = {
			id: streamId,
			user_id,
			track_ids: [],
			key,
			media,
		};
		this.streams.set(streamId, stream);
		log.debug("rtc", `initialized new stream ${streamId}`, media);
		return stream;
	}

	public acquireTransceiver(
		key: TrackKey,
		kind: MediaKind,
		encodings?: RTCRtpEncodingParameters[],
	): RTCRtpTransceiver {
		if (!this.rtc) throw new Error("RTCPeerConnection not initialized");

		for (const vt of this.localTransceivers) {
			if (vt.key === key && vt.transceiver.sender.track?.kind === kind) {
				return vt.transceiver;
			}
		}

		const tr = this.rtc.addTransceiver(kind, {
			direction: "inactive",
			sendEncodings: encodings,
		});
		log.info("rtc", "create transceiver", tr);

		this.localTransceivers.push({
			key,
			transceiver: tr,
		});

		if (tr.mid) {
			this.transceivers.set(tr.mid, { key, transceiver: tr });
		}

		return tr;
	}

	public subscribeToTracks(trackIds: string[]) {
		const add = trackIds.filter((id) => !this.subscribedTracks.has(id));
		if (add.length > 0) {
			for (const id of add) {
				this.subscribedTracks.add(id);
			}
			this.sendSignalling({
				type: "Subscribe",
				add,
				remove: [],
			});
		}
	}

	public unsubscribeFromTracks(trackIds: string[]) {
		const remove = trackIds.filter((id) => this.subscribedTracks.has(id));
		if (remove.length > 0) {
			for (const id of remove) {
				this.subscribedTracks.delete(id);
			}
			this.sendSignalling({
				type: "Subscribe",
				add: [],
				remove,
			});
		}
	}

	// TODO: public migrate(...) {}
}
