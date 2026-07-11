import { logger } from "@/utils/logger";

const voiceLog = logger.for("voice");

type LocalTrack = {
	track: MediaStreamTrack;
	stream: MediaStream;
	deviceId?: string;
};

type LocalScreen = {
	trackVideo: MediaStreamTrack;
	trackAudio: MediaStreamTrack | null;
	stream: MediaStream;
};

/** handles VoiceClient */
export class DeviceManager {
	private localMic: LocalTrack | null = null;
	private localCam: LocalTrack | null = null;
	private localScreenshare: LocalScreen | null = null;

	public async acquireMicrophone(deviceId?: string): Promise<LocalTrack> {
		if (this.localMic) {
			// we already have a microphone
			const l = this.localMic;

			// and the caller doesnt care about the device id or the device id is correct
			if (!deviceId || deviceId === l.deviceId) return this.localMic;
		}

		const stream = await navigator.mediaDevices.getUserMedia(
			deviceId ? { audio: { deviceId } } : { audio: true },
		);
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

		const stream = await navigator.mediaDevices.getUserMedia(
			deviceId ? { video: { deviceId } } : { video: true },
		);
		voiceLog.debug("got camera stream", stream);
		const track = stream.getTracks()[0];
		if (!track) throw new Error("todo: better error handling");
		const trackDeviceId = track.getSettings().deviceId;
		const l = { track, stream, deviceId: trackDeviceId };
		this.localCam = l;
		return l;
	}

	public async acquireScreenshare(): Promise<LocalScreen> {
		if (this.localScreenshare) return this.localScreenshare;

		const stream = await navigator.mediaDevices.getDisplayMedia({
			video: true,
			audio: true,
		});
		console.log("AAAAA", stream);

		// TODO: get metadata about the display stream
		// trackVideo.getSettings().displaySurface
		// trackVideo.label

		voiceLog.debug("got screenshare stream", stream);
		const trackVideo = stream.getVideoTracks()[0];
		if (!trackVideo) throw new Error("no video track in screenshare stream");
		console.log("AAA", trackVideo.label);
		const trackAudio = stream.getAudioTracks()[0] ?? null;
		const l = { trackVideo, trackAudio, stream };
		this.localScreenshare = l;
		trackVideo.addEventListener("ended", () => {
			this.localScreenshare = null;
		});
		return l;
	}

	public async enumerate(): Promise<MediaDeviceInfo[]> {
		const devices = await navigator.mediaDevices.enumerateDevices();
		return devices;
	}
}
