import type { MessageSync } from "ts-sdk";
import { useApi } from "@/api";

export function useSync(fn: (msg: MessageSync) => void) {
	const api = useApi();
	api.events.on("sync", ([msg]) => fn(msg));
}
