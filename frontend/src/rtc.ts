// TODO: move stuff here
const createClient = () => {
	let conn = new RTCPeerConnection(RTC_CONFIG);
	let reconnect = true;
	conn.addEventListener("connectionstatechange", () => {
		if (conn.connectionState === "closed") {
		}
	});

	return {
		conn,
		addTrack(track: MediaStreamTrack) {},
		stop() {
			reconnect = false;
		},
	};
};
