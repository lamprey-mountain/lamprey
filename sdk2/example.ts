import { Emitter, Lamprey, LampreyEvents } from "./src/index";

const lamprey = new Lamprey({
	apiUrl: "http://localhost:4000",
	// backend: "direct",
	// cache: "disable",
	// cdnUrl: "",
	// storage: "",
});

lamprey.on("messageCreate", (message) => {
	// TODO
});

lamprey.on("messageDelete", (channelId, messageId) => {
	// TODO
});

lamprey.start("token");

// console.log(await lamprey.rooms.fetch("room-id-here"));
