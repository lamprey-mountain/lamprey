export class JsonStream extends TransformStream<Uint8Array, unknown> {
	constructor() {
		const decoder = new TextDecoder();
		let buffer = "";

		super({
			transform(chunk, controller) {
				buffer += decoder.decode(chunk, { stream: true });

				while (true) {
					const endIdx = findJsonEnd(buffer);
					if (endIdx === -1) break;

					const raw = buffer.slice(0, endIdx);
					try {
						const obj = JSON.parse(raw);
						controller.enqueue(obj);
					} catch (e) {
						console.error("Failed to parse JSON segment:", e);
					}

					buffer = buffer.slice(endIdx).trimStart();
				}
			},
			flush(controller) {
				const final = decoder.decode();
				if (final.trim()) {
					try {
						controller.enqueue(JSON.parse(final));
					} catch {}
				}
			},
		});
	}
}

/**
 * Finds the end of the first complete JSON object in a string.
 * Returns -1 if the object is incomplete.
 */
function findJsonEnd(str: string): number {
	let braces = 0;
	let inString = false;
	let escaped = false;
	let started = false;

	for (let i = 0; i < str.length; i++) {
		const char = str[i];
		if (escaped) {
			escaped = false;
			continue;
		}
		if (char === "\\") {
			escaped = true;
			continue;
		}
		if (char === '"') {
			inString = !inString;
			continue;
		}
		if (inString) continue;

		if (char === "{") {
			braces++;
			started = true;
		} else if (char === "}") {
			braces--;
			if (started && braces === 0) return i + 1;
		}
	}
	return -1;
}
