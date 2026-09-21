import type { Stream, WebtransportClient } from "sdk";

// TODO: make configurable
const MAX_STREAM_CACHE = 5;

type StreamCacheEntry = {
	stream: Stream;
	id: string;
};

export class StreamManager {
	private roomStreams: StreamCacheEntry[] = [];
	private channelStreams: StreamCacheEntry[] = [];

	constructor(private client: WebtransportClient) {}

	subscribeRoom(room_id: string, onSync: (sync: any) => void): Stream {
		const existing = this.roomStreams.find((s) => s.id === room_id);
		if (existing) {
			// Move to end (most recent)
			this.roomStreams = [
				...this.roomStreams.filter((s) => s.id !== room_id),
				existing,
			];
			return existing.stream;
		}

		const stream = this.client.subscribeRoom({
			room_id,
			onSync,
		});

		this.roomStreams.push({ stream, id: room_id });

		if (this.roomStreams.length > MAX_STREAM_CACHE) {
			const oldest = this.roomStreams.shift();
			oldest?.stream.close();
		}

		return stream;
	}

	subscribeChannel(channel_id: string, onSync: (sync: any) => void): Stream {
		const existing = this.channelStreams.find((s) => s.id === channel_id);
		if (existing) {
			// Move to end (most recent)
			this.channelStreams = [
				...this.channelStreams.filter((s) => s.id !== channel_id),
				existing,
			];
			return existing.stream;
		}

		const stream = this.client.subscribeChannel({
			channel_id,
			onSync,
		});

		this.channelStreams.push({ stream, id: channel_id });

		if (this.channelStreams.length > MAX_STREAM_CACHE) {
			const oldest = this.channelStreams.shift();
			oldest?.stream.close();
		}

		return stream;
	}
}
