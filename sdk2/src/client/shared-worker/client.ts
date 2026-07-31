async function test() {
	const sessionId = "";

	// FIXME: handle compilation/bundling
	const worker = new SharedWorker("./worker.ts", {
		type: "module",
		name: `lamprey-client-${sessionId}`,
	});

	worker.addEventListener("error", (e) => {
		console.error("worker error", e.error);
	});

	worker.port.addEventListener(
		"message",
		(e: MessageEvent) => {},
		// (e: MessageEvent<WorkerEvent>) => {
		// 	const { data } = e;
		// 	if (data.type === "fetched") {
		// 		const promise = this.pendingRequests.get(data.nonce);
		// 		if (promise) {
		// 			if (data.body.ok) {
		// 				promise.resolve(data.body);
		// 			} else {
		// 				promise.reject(data.body);
		// 			}
		// 			this.pendingRequests.delete(data.nonce);
		// 		}
		// },
	);

	worker.port.start();

	// const timer = setInterval(() => {
	// 	this.post({ type: "heartbeat" });
	// }, HEATBEAT_INTERVAL_CLIENT);

	// // TODO: stop timer on disconnect
	// // clearInterval(timer);

	return {};
}
