// TODO: allow server specified ice servers
export const RTC_CONFIG: RTCConfiguration = {
	iceServers: [
		{ urls: ["stun:relay.webwormhole.io"] },
		{ urls: ["stun:stun.stunprotocol.org"] },
	],

	// FIXME: add iceTransportPolicy
	// ice failures under symmetric nat should NOT be silent
	// iceTransportPolicy: "...",
};
