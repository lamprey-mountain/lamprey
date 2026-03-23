// Represents an attribute line "a="
export type Attribute = {
	key: string;
	value?: string;
};

// Represents a connection line "c="
export type Connection = {
	netType: string; // e.g., "IN"
	addrType: string; // e.g., "IP4" or "IP6"
	address: string; // e.g., "203.0.113.1"
};

// Represents a media description line "m="
export type Media = {
	type: string; // e.g., "audio", "video"
	port: number;
	protocol: string; // e.g., "RTP/AVP"
	formats: string[]; // list of payload types or formats
	attributes: Attribute[]; // attributes specific to this media
	connection?: Connection; // optional media-level connection
	bandwidth?: string; // optional bandwidth line "b="
};

// Represents the origin line "o="
export type Origin = {
	username: string;
	sessionId: string;
	sessionVersion: string;
	netType: string;
	addrType: string;
	address: string;
};

// Represents the session-level information
export type Session = {
	version?: number;
	origin?: Origin;
	name?: string;
	connection?: Connection; // session-level connection
	bandwidth?: string; // session-level bandwidth "b="
};

// Represents timing information "t="
export type Timing = {
	startTime: string; // could also be number if you parse to integer
	stopTime: string;
};

// The top-level parsed SDP object
export type ParsedSessionDescription = {
	session: Session;
	timing: Timing | null;
	media: Media[];
	attributes: Attribute[]; // session-level attributes
	errors: string[]; // collected errors
};

export const parseSessionDescription = (
	s: string,
): ParsedSessionDescription => {
	const parsed: ParsedSessionDescription = {
		session: {},
		timing: null,
		media: [],
		attributes: [],
		errors: [],
	};

	let currentMedia: Media | undefined;

	const lines = s.trim().split("\r\n");
	for (let i = 0; i < lines.length; i++) {
		const match = lines[i].trim().match(/^([a-z])=(.*)$/);
		if (!match) {
			parsed.errors.push(
				`Line ${i}: Invalid format - ${JSON.stringify(lines[i])}`,
			);
			continue;
		}
		const [_, type, value] = match;
		switch (type) {
			case "v":
				parsed.session.version = parseInt(value, 10);
				break;
			case "o": {
				const [
					username,
					sessionId,
					sessionVersion,
					netType,
					addrType,
					address,
				] = value.split(" ");
				parsed.session.origin = {
					username,
					sessionId,
					sessionVersion,
					netType,
					addrType,
					address,
				};
				break;
			}
			case "s": {
				parsed.session.name = value;
				break;
			}
			case "c": {
				const [connNetType, connAddrType, connAddress] = value.split(" ");
				const connection = {
					netType: connNetType,
					addrType: connAddrType,
					address: connAddress,
				};
				if (currentMedia) {
					currentMedia.connection = connection;
				} else {
					parsed.session.connection = connection;
				}
				break;
			}
			case "t": {
				const [startTime, stopTime] = value.split(" ");
				parsed.timing = { startTime, stopTime };
				break;
			}
			case "m": {
				const [mediaType, port, protocol, ...formats] = value.split(" ");
				currentMedia = {
					type: mediaType,
					port: parseInt(port, 10),
					protocol,
					formats,
					attributes: [],
				};
				parsed.media.push(currentMedia);
				break;
			}
			case "a": {
				const [_0, _1, key, val] = value.match(/^(([a-z-]+):)?(.*)$/)!; // always parseable
				const attribute = key ? { key, value: val } : { key: val };
				if (currentMedia) {
					currentMedia.attributes.push(attribute);
				} else {
					parsed.attributes.push(attribute);
				}
				break;
			}
			case "b": {
				const bandwidth = value;
				if (currentMedia) {
					currentMedia.bandwidth = bandwidth;
				} else {
					parsed.session.bandwidth = bandwidth;
				}
				break;
			}
			default: {
				parsed.attributes.push({ key: `${type}-line`, value });
			}
		}
	}

	return parsed;
};

export const getAttributeDescription = (
	key: string,
	value: string | undefined,
) => {
	const descriptions: Record<string, string> = {
		"ice-ufrag": "ICE username fragment for authentication",
		"ice-pwd": "ICE password for authentication",
		"fingerprint": "Certificate fingerprint for DTLS",
		"setup": "DTLS setup role (active/passive/actpass)",
		"mid": "Media stream identification tag",
		"sendrecv": "Can send and receive media",
		"sendonly": "Can only send media",
		"recvonly": "Can only receive media",
		"inactive": "Media is inactive",
		"rtcp-mux": "RTP and RTCP multiplexed on same port",
		"rtcp-rsize": "Reduced-size RTCP packets allowed",
		"rtpmap": "RTP payload type mapping (codec info)",
		"fmtp": "Format-specific parameters",
		"rtcp-fb": "RTCP feedback mechanisms",
		"ssrc": "Synchronization source identifier",
		"ssrc-group": "SSRC grouping (like FID for RTX)",
		"msid": "Media stream and track identifiers",
		"candidate": "ICE connectivity check candidate (contains IP addresses)",
		"end-of-candidates": "No more ICE candidates will be sent",
		"extmap": "RTP header extension mapping",
		"extmap-allow-mixed": "Allow mixed one-byte and two-byte header extensions",
		"group": "Media grouping (like BUNDLE)",

		"nack": "Negative acknowledgment feedback",
		"pli": "Picture loss indication feedback",
		"fir": "Full intra request feedback",
		"goog-remb": "Google receiver estimated maximum bitrate",
		"transport-cc": "Transport-wide congestion control",
		"ccm": "Codec control messages",
		"apt": "Associated payload type (for RTX)",
		"ulpfec": "Uneven level protection forward error correction",
		"red": "Redundancy encoding",
		"rtx": "Retransmission payload type",
	};
	return descriptions[key] || "Custom attribute";
};
