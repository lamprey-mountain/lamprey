// TODO: use log.foo instead of console.foo

import { MessageClient, ScriptSubscribe, SignallingEvent, VoiceState, VoiceSubscription, } from "ts-sdk";
import { RTC_CONFIG } from "./util";
import { createEventHub, EventBus, EventHub } from "@solid-primitives/event-bus";
import { Accessor, createSignal, Setter } from "solid-js";
import { logger } from "@/utils/logger";
import { Api } from "@/api";
import { ReactiveSet } from "@solid-primitives/set";
import { ReactiveMap } from "@solid-primitives/map";

const log = logger.for("rtc");

/**
* - pending: needs to connect to sync websocket
* - connecting:
* - connected:
* - disconnected:
*/
export type VoiceConnectionState = "pending" | "connecting" | "connected" | "disconnected";

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
  key: string;
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
  // private transceivers = new Map<string, RTCRtpTransceiver>();

  constructor(
    public api: Api,
  ) {
    [this.connectionState, this.setConnectionState] = createSignal<VoiceConnectionState>("disconnected");
  }

  /** setup a new rtc peer connection */
  private init(): void {
    if (this.rtc) return;
    this.rtc = new RTCPeerConnection(RTC_CONFIG);
    this.setConnectionState(this.api.client.state.get() === "ready" ? "connecting" : "pending");

    // setup event listeners for rtc
    this.rtc.addEventListener("connectionstatechange", (e) => {
      // TODO: this.setConnectionState
    });

    this.rtc.addEventListener("iceconnectionstatechange", (e) => {
      // TODO: log.debug
      // TODO: restartIce() on conn.iceConnectionState === "failed"
    });

    this.rtc.addEventListener("signalingstatechange", (e) => {
      // TODO: log.debug
    });

    this.rtc.addEventListener("icegatheringstatechange", (e) => {
      // TODO: log.debug
    });

    this.rtc.addEventListener("icecandidate", (e) => {
      // TODO: log.debug
      // TODO: this.send
    });

    this.rtc.addEventListener("negotiationneeded", (e) => {
      // TODO: negotiate

      // console.info("[rtc:sdp] negotiation needed");
      // try {
      // 	makingOffer = true;
      // 	const offer = await conn.createOffer();
      // 	await conn.setLocalDescription(offer);
      // 	const tracks = getTrackMetadata();
      // 	console.info("[rtc:sdp] create offer", tracks);
      // 	send({
      // 		type: "Offer",
      // 		sdp: conn.localDescription?.sdp ?? "",
      // 		tracks,
      // 	});
      // } finally {
      // 	makingOffer = false;
      // }
    });

    this.rtc.addEventListener("track", (e) => {
      const t = e.transceiver;
      // TODO: handle track
    });

    this.rtc.addEventListener("datachannel", (e) => {
      // TODO: log.debug
    });

    // setup speaking indicator data channel
    const sc = this.rtc.createDataChannel("speaking", {
      ordered: false,
      protocol: "speaking",
      maxRetransmits: 0,
    });

    this.speaking.swapDatachannel(sc);

    // function reconnect() {
    // 	conn.close();
    // 	conn = new RTCPeerConnection(RTC_CONFIG);
    // 	ready = false;
    // 	chanSpeaking = undefined;
    // 	events.emit("reconnect", { conn });
    // 	setup();
    // }

  }

  public async connect(channelId: string): Promise<void> {
    this.init();

    const existing = this.api.voiceState;
    if (existing) {
      log.warn("signal", "already have a voice state, not resetting first", existing)
    }

    this.send({
      type: "VoiceState",
      state: {
        channel_id: channelId,
        self_mute: true,
        self_deaf: false,
        self_video: false,
        self_screen: false,
      },
    });
  }

  public disconnect(): void {
    this.setConnectionState("disconnected");
    this.send({
      type: "VoiceState",
      state: null,
    });
  }

  public send(msg: MessageClient): void {
    this.queue.push(msg);
    this.drainSendQueue();
  }

  // TEMP(?): public
  public drainSendQueue(): void {
    const currentUser = this.api.users.cache.get("@self");
    const user_id = currentUser?.id;
    if (!user_id) return;
    if (this.connectionState() === "pending") return;

    for (const msg of this.queue) {
      log.info("signal", "send " + msg.type, msg);
      this.api.client.send({
        type: "VoiceDispatch",
        user_id,
        payload: msg,
      });
    }

    this.queue.splice(0, this.queue.length);
  }

  public async handleVoiceState(uid: string, vs: VoiceState): Promise<void> {
    // TODO: handle disconnects
    // 	if (!e.state) {
    // 		console.log("[rtc:stream] clean up tracks from", e.user_id);
    // 		const filtered = remoteStreams.filter((s) => s.user_id !== e.user_id);
    // 		remoteStreams.splice(0, remoteStreams.length, ...filtered);
    // 		for (const [key, s] of streams) {
    // 			if (s.user_id === e.user_id) streams.delete(key);
    // 		}
    // 	}
  }

  public async handleSignalingEvent(msg: SignallingEvent): Promise<void> {
    if (!this.rtc) return;

    switch (msg.type) {
      case "Connected": {
        this.setConnectionState("connected");
        this.drainSendQueue(); // NOTE: is this necessary? is it even possible to have a stuck queue when receiving Connected?
        // TODO: send want subscriptions?
        break;
      }
      case "Offer": {
        // await this.pc.setRemoteDescription({ type: "offer", sdp: message.sdp });
        // const answer = await this.pc.createAnswer();
        // await this.pc.setLocalDescription(answer);
        // this.sendSignaling({ type: "Answer", sdp: answer.sdp ?? "" });
        // TODO

        // const readyForOffer =
        // 	!makingOffer &&
        // 	(conn.signalingState === "stable" || settingRemoteAnswer);
        // if (!readyForOffer) {
        // 	console.log(
        // 		"[rtc:sdp] ignore server offer; signallingState=",
        // 		conn.signalingState,
        // 	);
        // 	return;
        // }

        // console.log("[rtc:sdp] accept offer; create answer");
        // try {
        // 	await conn.setRemoteDescription({
        // 		type: "offer",
        // 		sdp: msg.sdp,
        // 	});
        // 	await conn.setLocalDescription(await conn.createAnswer());
        // 	send({ type: "Answer", sdp: conn.localDescription?.sdp ?? "" });
        // } catch (err) {
        // 	console.error("[rtc:sdp] error while accepting offer", err);
        // 	console.log("COPY PASTE THIS", {
        // 		localDescription: conn.localDescription,
        // 		answer: msg.sdp,
        // 	});
        // }

        break;
      }
      case "Answer": {
        await this.rtc.setRemoteDescription({ type: "answer", sdp: msg.sdp });

        // TODO
        // if (conn.signalingState !== "have-local-offer") {
        // 	console.log(
        // 		"[rtc:sdp] ignoring unexpected answer, state:",
        // 		conn.signalingState,
        // 	);
        // 	return;
        // }

        // console.log("[rtc:sdp] accept answer");
        // try {
        // 	settingRemoteAnswer = true;
        // 	await conn.setRemoteDescription({
        // 		type: "answer",
        // 		sdp: msg.sdp,
        // 	});
        // } catch (err) {
        // 	console.error("[rtc:sdp] error while accepting answer", err);
        // 	console.log("COPY PASTE THIS", {
        // 		answer: msg.sdp,
        // 		localDescription: conn.localDescription,
        // 	});
        // } finally {
        // 	settingRemoteAnswer = false;
        // }
        break;
      }
      case "Candidate": {
        console.log("[rtc:signal] remote ICE candidate", msg.candidate);
        await this.rtc.addIceCandidate({ candidate: msg.candidate });
        break;
      }
      case "Have": {
        // this.processHaveMessage(message.user_id, message.tracks);

        // TODO
        // const currentUser = api2.users.cache.get("@self");
        // const user_id = currentUser?.id;
        // const ruid = msg.user_id;
        // if (ruid === user_id) {
        // 	console.log("[rtc:signal] ignoring Have from self");
        // 	return;
        // }

        // console.group("[rtc:stream] process Have");
        // console.log("[rtc:signal] got Have from %s", ruid, msg.tracks);
        // console.log(
        // 	"[rtc:signal] current transceivers",
        // 	conn.getTransceivers().map((t) => [t.mid, t.direction]),
        // );
        // for (const track of msg.tracks) {
        // 	const streamId = `${ruid}:${track.key}`;
        // 	let s = remoteStreams.find((s) => s.id === streamId);
        // 	if (s) {
        // 		console.debug("[rtc:stream] reuse stream %s", streamId, s);
        // 		if (!s.mids.includes(track.mid)) s.mids.push(track.mid);
        // 	} else {
        // 		const media = new MediaStream();
        // 		console.log("[rtc:stream] initialized new stream", streamId, media);
        // 		s = {
        // 			id: streamId,
        // 			user_id: ruid,
        // 			mids: [track.mid],
        // 			key: track.key,
        // 			media,
        // 		};
        // 		remoteStreams.push(s);
        // 		streams.set(streamId, s);
        // 	}

        // 	// create a stream from mids
        // 	for (const mid of s.mids) {
        // 		const tn = transceivers.get(mid);
        // 		if (tn) {
        // 			const tr = tn.receiver.track;
        // 			s.media.addTrack(tr);
        // 			console.log(
        // 				"[rtc:stream] (re)added track %s (kind %s) to stream %s",
        // 				mid,
        // 				tr.kind,
        // 				streamId,
        // 			);
        // 		} else {
        // 			console.log(
        // 				"[rtc:stream] missing transceiver, will wait for track event",
        // 			);
        // 		}
        // 	}

        // 	// update streams for reactivity
        // 	streams.set(streamId, { ...s });

        // 	console.log(
        // 		"[rtc:state] current remoteStreams, transceivers:",
        // 		remoteStreams,
        // 		transceivers,
        // 	);
        // }
        // console.groupEnd();
        break;
      }
      case "Want": {
        // TODO: update sinks
        break;
      }
      case "Migrate": {
        // TODO: reconnect
        break;
      }
      case "Error": {
        log.error("signal", "got SignallingEvent::Error", msg)
        break;
      }
      default: {
        console.warn("[rtc:signal] unknown voice dispatch ", msg);
      }
    }
  }

  public setSubscriptions(subs: Array<VoiceSubscription>): void {
    // TODO
  }

  public acquireTransceiver(
    key: string,
    kind: "audio" | "video",
    encodings?: RTCRtpEncodingParameters[],
  ): RTCRtpTransceiver {
    if (!this.rtc) throw "better errors";
    // TODO: deduplicate by key
    // const s = localStreams.find((s) => s.key === stream);
    // if (!s) throw new Error("could not find that local stream");

    const tr = this.rtc.addTransceiver(kind, {
      direction: "inactive",
      sendEncodings: encodings,
    });
    log.info("rtc", "create transceiver", tr);
    // this.transceivers.push(tr);
    return tr;
  }

  // NOTE: unsure what this is for exactly
  public getLocalStream(key: string) {
    const currentUser = this.api.users.cache.get("@self");
    const user_id = currentUser?.id;
    if (!user_id) return;

    const existing = this.localStreams.find(
      (i) => i.key === key && i.user_id === user_id,
    );
    if (existing) {
      console.log("[rtc:local] reuse local stream", key, existing);
      return existing;
    }
    console.log("[rtc:local] create local stream", key);
    const media = new MediaStream();
    this.localStreams.push({
      id: `${user_id}:${key}`,
      user_id,
      transceivers: [],
      key,
      media,
    });
  }

  // 	updateIndicators(indicators: Indicators) {
  // 		const existing = untrack(() => api2.voiceState);
  // 		if (!existing) return;
  // 		const unchanged =
  // 			existing.self_deaf === indicators.self_deaf &&
  // 			existing.self_mute === indicators.self_mute &&
  // 			existing.self_video === indicators.self_video &&
  // 			existing.self_screen === indicators.self_screen;
  // 		if (unchanged) return;
  // 		send({
  // 			type: "VoiceState",
  // 			state: {
  // 				thread_id: existing.thread_id ?? existing.channel_id,
  // 				...indicators,
  // 			},
  // 		});
  // 	},

  private migrate() {
    // TODO: how does this work?
    // 1. create new rtc instance
    // 2. recreate existing transceivers on new rtc instance
    // 3. close old rtc instance
  }
}

export class Speaking {
  public users = new ReactiveMap<string, { flags: number }>();
  private timeouts = new Map();
  private sc: RTCDataChannel | null = null;

  swapDatachannel(
    sc: RTCDataChannel,
  ) {
    sc.addEventListener("close", () => {
      // TODO: log.info
      // log.info("speaking", "channel closed", null);
      // TODO: reconnect?
    });

    sc.addEventListener("error", (e) => {
      // TODO: log.error
      console.error("[rtc:speaking] speaking channel error", e.error);
    });

    sc.addEventListener("open", () => {
      // TODO: log.info
      // console.log("[rtc:vad] speaking channel opened");

      // if (this.chanSpeaking) {
      // 	console.warn("[rtc:speaking] already have a speaking channel");
      // }

      // TODO
      // this.sc = sc;
    });

    sc.addEventListener("message", (e) => {
      const { user_id, flags } = JSON.parse(e.data);
      // TODO: log.debug
      // console.debug("[rtc:speaking] recv speaking", { user_id, flags });

      // TODO: handle speaking
      // clearTimeout(speaking.get(user_id)?.timeout);
      // const timeout = setTimeout(() => speaking.delete(user_id), 10 * 1000);
      // speaking.set(user_id, { flags, timeout });
    })
  }

  public send(flags: number) {
    // log.debug("speaking", "send", flags);
    // this.sc?.send(JSON.stringify({ flags }));
    // TODO
  }
}
