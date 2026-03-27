import { createResource, For } from "solid-js";
import { useApi2 } from "@/api";
import { UserView } from "./User";
import { type UserWithRelationship } from "sdk";

export function UserProfile(props: { user: UserWithRelationship }) {
	const api2 = useApi2();

	const [mutualRooms] = createResource(
		() => props.user.id,
		async (user_id) => {
			const { data } = await api2.client.http.GET(
				"/api/v1/user/{user_id}/room",
				{
					params: {
						path: { user_id },
						query: { pagination: {} },
					},
				},
			);
			return data;
		},
	);

	return (
		<div class="user-profile-page">
			<UserView user={props.user} />
			<br />
			<h3 class="dim">mutual rooms</h3>
			<ul style="list-style: disc inside">
				{/* TODO: use actual store/live update */}
				<For each={mutualRooms()?.items ?? []} fallback="no mutual rooms :(">
					{(room) => (
						<li>
							<a href={`/room/${room.id}`}>{room.name}</a>
						</li>
					)}
				</For>
			</ul>
		</div>
	);
}
