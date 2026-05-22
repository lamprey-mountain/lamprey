import { MessageSync } from "./ts-sdk/index"

const RTC_CONFIG: RTCConfiguration = {
  iceServers: [
    { urls: ["stun:relay.webwormhole.io"] },
    { urls: ["stun:stun.stunprotocol.org"] },
  ],
};

let rtcOut = new RTCPeerConnection(RTC_CONFIG);
let rtcIn = new RTCPeerConnection(RTC_CONFIG);

function handleEvent(sync: MessageSync) {
  if (sync.type === "VoiceDispatch") {
    const t = sync.payload.type;
    if (t === "Ready") {

    } else if (t === "Offer") {

    } else {
      // ...
    }
  } else if (sync.type === "VoiceState") {

  }
}

