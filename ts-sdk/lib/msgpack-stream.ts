import { Unpackr } from "msgpackr";

export class MsgpackStream extends TransformStream<Uint8Array, unknown> {
	constructor() {
		const unpacker = new Unpackr({
			mapsAsObjects: true,
		});

		let buffer = new Uint8Array(0);

		super({
			transform(chunk, controller) {
				const combined = new Uint8Array(buffer.length + chunk.length);
				combined.set(buffer);
				combined.set(chunk, buffer.length);
				buffer = combined;

				let consumedUntil = 0;

				try {
					unpacker.unpackMultiple(buffer, (value, _start, end) => {
						controller.enqueue(value);
						if (end !== undefined) {
							consumedUntil = end;
						} else console.warn("no end!");
					});

					buffer = buffer.slice(consumedUntil);
				} catch (_e) {
					// if it throws, we have a partial message
					if (consumedUntil > 0) {
						buffer = buffer.slice(consumedUntil);
					}
				}
			},
			flush() {
				buffer = new Uint8Array(0);
			},
		});
	}
}
