import { ReactiveMap } from "@solid-primitives/map";
import { bytesToUuid, uuidToBytes } from "@/utils/uuid";
import { log } from "./VoiceClient";

export class Speaking {
	public users = new ReactiveMap<string, { flags: number }>();
	private timeouts = new Map<string, ReturnType<typeof setTimeout>>();
	private sc: RTCDataChannel | null = null;

	swapDataChannel(sc: RTCDataChannel) {
		sc.binaryType = "arraybuffer";
		sc.addEventListener("close", () => {
			log.info("speaking", "channel closed", null);
		});

		sc.addEventListener("error", (e) => {
			log.error("speaking", "speaking channel error", e);
		});

		// FIXME: this.sc is never set if open is never set
		// FIXME: race condition if sc is swapped but then the old sc opens
		// FIXME: resource leak, sc is never closed when swapped
		// FIXME: clear sc on error/close
		// FIXME: clear pending timeouts when sc closes
		sc.addEventListener("open", () => {
			log.info("speaking", "channel opened", null);
			if (this.sc) {
				log.warn("speaking", "already have a speaking channel", null);
				// TODO: consider closing the existing channel
			}
			this.sc = sc;
		});

		sc.addEventListener("message", (e) => {
			const data = new Uint8Array(e.data);
			// expects 33 bytes (16 bytes source_mid + 1 byte flags + 16 bytes user_id)
			if (data.length !== 33) {
				log.warn(
					"speaking",
					"invalid binary speaking data length",
					data.length,
				);
				return;
			}
			const flags = data[16];
			const userId = bytesToUuid(data.slice(17, 33));

			log.debug("speaking", "recv speaking", { userId, flags });

			clearTimeout(this.timeouts.get(userId));
			const timeout = setTimeout(() => {
				this.users.delete(userId);
				this.timeouts.delete(userId);
			}, 10 * 1000);
			this.timeouts.set(userId, timeout);
			this.users.set(userId, { flags });
		});
	}

	public send(mid: string, flags: number) {
		log.debug("speaking", "send", { mid, flags });
		const bytes = new Uint8Array(17);
		bytes.set(uuidToBytes(mid), 0);
		bytes[16] = flags;
		this.sc?.send(bytes);
	}
}
