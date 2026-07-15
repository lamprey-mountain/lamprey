// TODO: allow server specified ice servers
export const RTC_CONFIG: RTCConfiguration = {
	iceServers: [
		{ urls: "stun:stun.l.google.com:19302" },
		{ urls: "stun:stun1.l.google.com:19302" },
	],

	// FIXME: add iceTransportPolicy
	// ice failures under symmetric nat should NOT be silent
	// iceTransportPolicy: "...",
};
