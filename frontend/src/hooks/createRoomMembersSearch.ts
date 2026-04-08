import { createResource, createSignal } from "solid-js";
import { useApi } from "@/api";

export function createRoomMembersSearch(room_id: () => string) {
	const api2 = useApi();

	// TODO: debounce queries
	// TODO: react to sync events

	const [searchQuery, setSearch] = createSignal({ query: "", limit: 20 });
	const [search] = createResource(
		() => [searchQuery(), room_id()!] as const,
		async ([query, room_id]) => {
			const { data } = await api2.client.http.GET(
				"/api/v1/room/{room_id}/member/search",
				{
					params: { path: { room_id }, query },
				},
			);
			return data;
		},
	);

	return [search, setSearch] as const;
}
