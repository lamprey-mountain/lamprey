import { useApi } from "@/api";
import { MessageSync } from "ts-sdk";

export function useSync(fn: (msg: MessageSync) => void) {
	const api = useApi();
	api.events.on("sync", ([msg]) => fn(msg));
}
