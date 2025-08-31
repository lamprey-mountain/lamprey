import { RouteSectionProps, useNavigate } from "@solidjs/router";
import { createResource, For, Match, Show, Switch } from "solid-js";
import { useApi } from "./api";
import { UserView } from "./User";
import { type UserWithRelationship } from "sdk";
import { useCtx } from "./context";

export function UserProfile(props: RouteSectionProps) {
	const api = useApi();
	const ctx = useCtx();
	const nav = useNavigate();

	const [user, { refetch }] = createResource(
		() => props.params.user_id,
		async (userId) => {
			const { data } = await api.client.http.GET("/api/v1/user/{user_id}", {
				params: { path: { user_id: userId } },
			});
			return data as UserWithRelationship;
		},
	);

	const relationship = () => user()?.relationship;

	const createDm = async () => {
		const { data } = await api.client.http.POST(
			"/api/v1/user/@self/dm/{target_id}",
			{
				params: { path: { target_id: props.params.user_id } },
			},
		);
		if (data) {
			nav(`/thread/${data.id}`);
		}
	};

	const sendFriendRequest = async () => {
		await api.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id: props.params.user_id } },
		});
		refetch();
	};

	const removeFriend = async () => {
		await api.client.http.DELETE("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id: props.params.user_id } },
		});
		refetch();
	};

	const blockUser = async () => {
		await api.client.http.PUT("/api/v1/user/@self/block/{target_id}", {
			params: { path: { target_id: props.params.user_id } },
		});
		refetch();
	};

	const unblockUser = async () => {
		await api.client.http.DELETE("/api/v1/user/@self/block/{target_id}", {
			params: { path: { target_id: props.params.user_id } },
		});
		refetch();
	};

	const [mutualRooms] = createResource(
		() => props.params.user_id,
		async (user_id) => {
			const { data } = await api.client.http.GET(
				"/api/v1/user/{user_id}/room",
				{
					params: { path: { user_id } },
				},
			);
			return data;
		},
	);

	return (
		<Show when={user()}>
			<div class="user-profile">
				<UserView user={user()!} />
				<Show when={user()?.description}>
					<p>{user()?.description}</p>
				</Show>
				<div class="actions">
					<button onClick={createDm}>Create DM</button>
					<Switch>
						<Match when={relationship()?.relation === "Friend"}>
							<button onClick={removeFriend}>Remove Friend</button>
						</Match>
						<Match when={relationship()?.relation === "Outgoing"}>
							<button onClick={removeFriend}>Cancel Request</button>
						</Match>
						<Match when={relationship()?.relation === "Incoming"}>
							<button onClick={sendFriendRequest}>Accept Friend</button>
						</Match>
						<Match when={!relationship()?.relation}>
							<button onClick={sendFriendRequest}>Add Friend</button>
						</Match>
					</Switch>
					<Switch>
						<Match when={relationship()?.relation === "Block"}>
							<button onClick={unblockUser}>Unblock</button>
						</Match>
						<Match when={relationship()?.relation !== "Block"}>
							<button onClick={blockUser}>Block</button>
						</Match>
					</Switch>
				</div>
				<b>mutual rooms</b>
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
		</Show>
	);
}
