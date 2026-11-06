/** decode unpadded url safe base64 */
export function base64UrlToBytes(str: string): Uint8Array<ArrayBuffer> {
	const b64 = str.replace(/-/g, "+").replace(/_/g, "/");
	const pad = b64.length % 4;
	const binary = atob(pad ? b64 + "=".repeat(4 - pad) : b64);

	// explicitly backed by an ArrayBuffer so the result is a `BufferSource`
	const bytes = new Uint8Array(new ArrayBuffer(binary.length));
	for (let i = 0; i < binary.length; i++) {
		bytes[i] = binary.charCodeAt(i);
	}

	return bytes;
}

/** encode bytes as unpadded url safe base64 */
export function bytesToBase64Url(bytes: Uint8Array): string {
	let binary = "";
	for (const byte of bytes) {
		binary += String.fromCharCode(byte);
	}

	return btoa(binary)
		.replace(/\+/g, "-")
		.replace(/\//g, "_")
		.replace(/=+$/, "");
}
