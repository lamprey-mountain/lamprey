import { logger } from "@/utils/logger";
import { VoiceClient } from "./VoiceClient";

const voiceLog = logger.for("voice");

type LocalTrack = {
  track: MediaStreamTrack;
  stream: MediaStream;
  deviceId?: string,
}

type LocalScreen = {
  trackVideo: MediaStreamTrack;
  trackAudio: MediaStreamTrack;
  stream: MediaStream;
  // TODO: ???
}

/** handles VoiceClient */
export class DeviceManager {
  private localMic: LocalTrack | null = null;
  private localCam: LocalTrack | null = null;
  private localScreenshare: LocalScreen | null = null;

  constructor(
    // private voiceClient: VoiceClient,
  ) { }

  public async acquireMicrophone(deviceId?: string): Promise<LocalTrack> {
    if (this.localMic) {
      // we already have a microphone
      const l = this.localMic;

      // and the caller doesnt care about the device id or the device id is correct
      if (!deviceId || deviceId === l.deviceId) return this.localMic;
    }

    const stream = await navigator.mediaDevices.getUserMedia(deviceId ? { audio: { deviceId } } : { audio: true });
    voiceLog.debug("got microphone stream", stream);
    const track = stream.getTracks()[0];
    if (!track) throw new Error("todo: better error handling");
    const trackDeviceId = track.getSettings().deviceId;
    const l = { track, stream, deviceId: trackDeviceId };
    this.localMic = l;
    return l;
  }

  public async acquireCamera(deviceId?: string): Promise<LocalTrack> {
    if (this.localCam) {
      // we already have a camera
      const l = this.localCam;

      // and the caller doesnt care about the device id or the device id is correct
      if (!deviceId || deviceId === l.deviceId) return this.localCam;
    }

    const stream = await navigator.mediaDevices.getUserMedia(deviceId ? { video: { deviceId } } : { video: true });
    voiceLog.debug("got camera stream", stream);
    const track = stream.getTracks()[0];
    if (!track) throw new Error("todo: better error handling");
    const trackDeviceId = track.getSettings().deviceId;
    const l = { track, stream, deviceId: trackDeviceId };
    this.localCam = l;
    return l;
  }

  public async acquireScreensare(): Promise<MediaStream> {
    return await navigator.mediaDevices.getDisplayMedia({
      video: true,
      audio: true,
    });
  }

  public async enumerate(): Promise<MediaDeviceInfo[]> {
    const devices = await navigator.mediaDevices.enumerateDevices();
    return devices;
  }
}
