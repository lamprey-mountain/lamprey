import { ReactiveMap } from "@solid-primitives/map";
import { createContext, ParentProps, useContext } from "solid-js";
import { createStore } from "solid-js/store";
import { DeviceManager } from "./DeviceManager";
import { logger } from "@/utils/logger";
import { VoiceClient, VoiceConnectionState } from "./VoiceClient";
import { createVAD } from "./vad";
import { useApi } from "@/api";

type VoiceActions = {
  selectChannel: (channelId: string) => Promise<void>;
  disconnect: () => void;
  toggleMicrophone: (enabled?: boolean) => Promise<void>;
  toggleCamera: (enabled?: boolean) => Promise<void>;
  toggleScreenshare: () => Promise<void>;
  startScreenshare: () => Promise<void>;
  stopScreenshare: () => Promise<void>;
  toggleDeafened: (enabled?: boolean) => Promise<void>;
  playMusic: () => Promise<void>;
  subscribeToParticipant: (userId: string, trackKey: "user" | "screen", subscribe: boolean) => void;
}

type VoiceProviderState = {
  vc: VoiceClient,
  connectionState: VoiceConnectionState;
  joinedChannelId: string | null;
  muted: boolean;
  deafened: boolean;
  camera: boolean;
  screensharing: boolean;
  musicing: boolean;
  hasVoiceActivity: boolean;
  participants: ReactiveMap<string, VoiceConfigUser>;
}

type VoiceConfigUser = {
  volume: number;
  mute: boolean;
  mute_video: boolean;
};


const VoiceContext = createContext<[VoiceProviderState, VoiceActions]>();

const voiceLog = logger.for("voice");
const rtcLog = logger.for("rtc");

export const VoiceProvider = (props: ParentProps<{}>) => {
  const api = useApi();
  const vc = new VoiceClient(api);
  const devices = new DeviceManager();
  const vad = createVAD();

  const [store, update] = createStore<VoiceProviderState>({
    vc,
    joinedChannelId: null,
    muted: false,
    deafened: false,
    camera: false,
    screensharing: false,
    musicing: false,
    get hasVoiceActivity() {
      return vad.hasVoiceActivity();
    },
    get connectionState() {
      return vc.connectionState();
    },
    participants: new ReactiveMap(),
  });

  api.events.on("sync", ([sync, _envelope]) => {
    if (sync.type === "VoiceDispatch") {
      vc.handleSignalingEvent(sync.payload);
    } else if (sync.type === "VoiceState") {
      // TODO
    }
  });

  api.events.on("ready", () => {
    vc.drainSendQueue();
  });

  const actions: VoiceActions = {
    async selectChannel(channelId: string) {
      // vc.connect(channelId);
      // TODO
    },

    disconnect() {
      vc.disconnect();
    },

    async toggleMicrophone(enabled_?: boolean) {
      // get microphone
      const mic = await devices.acquireMicrophone();

      // enable/disable it as indicated
      const enabled = enabled_ ?? !mic.track.enabled;
      mic.track.enabled = enabled;

      // connect to transceiver
      const tr = vc.acquireTransceiver("user", "audio");
      if (tr.currentDirection !== "stopped") {
        await tr.sender.replaceTrack(mic.track);
        tr.direction = "sendonly";
      } else {
        voiceLog.warn("microphone transceiver is stopped", tr);
      }

      // connect to vad
      vad.connect(mic.stream);

      // update muted state
      update("muted", !enabled);
    },

    async toggleCamera(enabled?: boolean) {
      // TODO
    },

    async toggleScreenshare() {
      // TODO: if screensharing stop otherwise start
    },

    async startScreenshare() {
      // TODO
    },

    async stopScreenshare() {
      // TODO
    },

    async toggleDeafened(enabled?: boolean) {
      // TODO
    },

    async playMusic() {
      // TODO
    },

    subscribeToParticipant(userId: string, trackKey: "user" | "screen", subscribe: boolean) {
      // TODO
    },
  };

  return (
    <VoiceContext.Provider value={[store, actions]}>
      {props.children}
    </VoiceContext.Provider>
  );
}

export const useVoice = () => {
  const context = useContext(VoiceContext);
  if (!context) throw new Error("useVoice must be used within a VoiceProvider");
  return context;
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

async function loadMusic() {
  const audio = document.createElement("audio");
  audio.src =
    "https://dump.celery.eu.org/resoundingly-one-bullsnake.opus";
  audio.crossOrigin = "anonymous";
  await new Promise((res) =>
    audio.addEventListener("loadedmetadata", res, { once: true }),
  );
  const stream: MediaStream =
    "captureStream" in audio
      ? (
        audio as HTMLAudioElement & { captureStream: () => MediaStream }
      ).captureStream()
      : (
        audio as HTMLAudioElement & {
          mozCaptureStream: () => MediaStream;
        }
      ).mozCaptureStream();
  const track = stream.getAudioTracks()[0];
  return { track, stream };
}
