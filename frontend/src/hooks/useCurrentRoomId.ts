import { useParams } from "@solidjs/router";
import { useChannels } from "@/api";

export const useCurrentRoomId = () => {
	const params = useParams();
	const channels = useChannels();
	const channel = channels.use(() => params.channel_id);

	// solidjs doesnt like it if i return null/undefined here
	return () => params.room_id ?? channel()?.room_id;
};
