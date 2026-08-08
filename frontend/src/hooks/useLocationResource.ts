import { useParams } from "@solidjs/router";
import { useApi } from "@/api";

export const useLocationResource = () => {
	const api = useApi();
	const params = useParams();

	const channelId = (): string | undefined => params.channel_id;
	const roomId = (): string | undefined => params.room_id;

	const channel = api.channels.use(channelId);
	const room = api.rooms.use(roomId);

	return {
		get channelId() {
			return channelId();
		},
		get channel() {
			return channel();
		},
		get roomId() {
			return roomId();
		},
		get room() {
			return room();
		},
	};
};
