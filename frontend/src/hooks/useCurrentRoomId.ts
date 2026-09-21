import { useParams } from "@solidjs/router";
import { useApi } from "@/api";

export const useCurrentRoomId = () => {
	const params = useParams();
	const api = useApi();
	const channel = api.channels.use(() => params.channel_id);

	// solidjs doesnt like it if i return null/undefined here
	return () => params.room_id ?? channel()?.room_id ?? "";
};
