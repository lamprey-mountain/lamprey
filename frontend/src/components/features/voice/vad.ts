import { createSignal, onCleanup } from "solid-js";
// @ts-expect-error
import vadProcessorUrl from "@/components/features/voice/VADProcessor?url";
import { logger } from "@/utils/logger";

// TODO: investigate more ways to debounce
// deep neural network: https://www.microsoft.com/en-us/research/wp-content/uploads/2017/04/Tashev-Mirsamadi_DNN-based-Causal-VAD.pdf
// another implementation: https://github.com/snakers4/silero-vad
export const createVAD = () => {
	const vadLog = logger.for("vad");
	vadLog.debug("init");

	const [hasVoiceActivity, setHasVoiceActivity] = createSignal(false);
	const ctx = new AudioContext();
	let source: MediaStreamAudioSourceNode | undefined;
	let node: AudioWorkletNode | undefined;

	const initWorklet = async () => {
		try {
			await ctx.audioWorklet.addModule(vadProcessorUrl);
			node = new AudioWorkletNode(ctx, "vad-processor");
			node.port.onmessage = (event) => {
				if (event.data && typeof event.data.hasVoiceActivity === "boolean") {
					setHasVoiceActivity(event.data.hasVoiceActivity);
				}
			};
			if (source) {
				source.connect(node);
			}
		} catch (e) {
			vadLog.error("failed to initialize audio worklet", e);
		}
	};

	initWorklet();

	onCleanup(() => {
		vadLog.debug("cleanup");
		node?.disconnect();
		source?.disconnect();
		ctx.close();
	});

	return {
		hasVoiceActivity,
		// connect(track: MediaTrack) {},
		connect(stream: MediaStream) {
			source?.disconnect();
			source = ctx.createMediaStreamSource(stream);
			if (node) {
				source.connect(node);
			}
			if (ctx.state === "suspended") {
				ctx.resume();
			}
			vadLog.debug("new stream connected");
		},
	};
};
