// @ts-ignore
class VADProcessor extends AudioWorkletProcessor {
	private threshold = 0.02;
	private minFramesEnable = 3;
	private minFramesDisable = 5;
	private consecutiveOn = 0;
	private consecutiveOff = 0;
	private hasVoiceActivity = false;

	process(
		inputs: Float32Array[][],
		_outputs: Float32Array[][],
		_parameters: Record<string, Float32Array>,
	) {
		const input = inputs[0];
		if (input.length > 0 && input[0].length > 0) {
			const channel = input[0];
			let sumSquares = 0;
			for (let i = 0; i < channel.length; i++) {
				sumSquares += channel[i] * channel[i];
			}
			const rms = Math.sqrt(sumSquares / channel.length);
			const currentActivity = rms > this.threshold;

			if (currentActivity) {
				this.consecutiveOn++;
				this.consecutiveOff = 0;
				if (
					!this.hasVoiceActivity && this.consecutiveOn >= this.minFramesEnable
				) {
					this.hasVoiceActivity = true;
					(this as any).port?.postMessage({ hasVoiceActivity: true });
				}
			} else {
				this.consecutiveOff++;
				this.consecutiveOn = 0;
				if (
					this.hasVoiceActivity && this.consecutiveOff >= this.minFramesDisable
				) {
					this.hasVoiceActivity = false;
					(this as any).port?.postMessage({ hasVoiceActivity: false });
				}
			}
		}
		return true;
	}
}

// @ts-ignore
registerProcessor("vad-processor", VADProcessor);
export {};
