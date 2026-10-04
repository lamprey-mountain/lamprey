/// <reference types="@types/audioworklet" />

// TODO: see if theres a better method of VAD
class VADProcessor extends AudioWorkletProcessor {
	private onThreshold = 0.02;
	private offThreshold = 0.012; // hysteresis
	private enableFrames = Math.round((20 * sampleRate) / 1000 / 128); // ~20ms
	private disableFrames = Math.round((250 * sampleRate) / 1000 / 128); // ~250ms hangover
	private reportEvery = Math.round((33 * sampleRate) / 1000 / 128); // ~30Hz meter
	private on = 0;
	private off = 0;
	private active = false;
	private frame = 0;
	private peak = 0;

	process(
		inputs: Float32Array[][],
		_outputs: Float32Array[][],
		_parameters: Record<string, Float32Array>,
	) {
		const ch = inputs[0]?.[0];
		if (!ch?.length) return true;

		// calculate voice activity via root mean square
		let sum = 0;
		for (let i = 0; i < ch.length; i++) sum += ch[i] * ch[i];
		const rms = Math.sqrt(sum / ch.length);
		this.peak = Math.max(this.peak, rms);

		const thr = this.active ? this.offThreshold : this.onThreshold;
		if (rms > thr) {
			this.on++;
			this.off = 0;
		} else {
			this.off++;
			this.on = 0;
		}

		let changed = false;
		if (!this.active && this.on >= this.enableFrames) {
			this.active = true;
			changed = true;
		} else if (this.active && this.off >= this.disableFrames) {
			this.active = false;
			changed = true;
		}

		if (changed || ++this.frame >= this.reportEvery) {
			this.port.postMessage({ hasVoiceActivity: this.active, rms: this.peak });
			this.peak = 0;
			this.frame = 0;
		}

		return true;
	}
}

registerProcessor("vad-processor", VADProcessor);

export {};
