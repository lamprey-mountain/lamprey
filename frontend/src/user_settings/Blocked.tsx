import { For, Show, type VoidProps } from "solid-js";
import { type Pagination, type RelationshipWithUserId, type User } from "sdk";
import { useApi } from "../api.tsx";
import { createResource } from "solid-js";
import { Avatar } from "../User.tsx";
import { useCtx } from "../context.ts";

function BlockedUserEntry(
	props: {
		relationship: RelationshipWithUserId;
		onUnblock: (userId: string) => void;
	},
) {
	const api = useApi();
	const user = api.users.fetch(() => props.relationship.user_id);

	return (
		<Show when={user()}>
			{(u) => (
				<li class="blocked-user-item">
					<Avatar user={u()} />
					<span class="name">{u().name}</span>
					<button onClick={() => props.onUnblock(u().id)}>Unblock</button>
				</li>
			)}
		</Show>
	);
}

export function Blocked(_props: VoidProps<{ user: User }>) {
	const api = useApi();
	const ctx = useCtx();

	const [blockedUsers, { refetch }] = createResource(async () => {
		const { data, error } = await api.client.http.GET(
			"/api/v1/user/@self/block",
			{
				params: { query: { limit: 100 } },
			},
		);
		if (error) {
			throw error;
		}
		return data as Pagination<RelationshipWithUserId>;
	});

	const unblockUser = (userId: string) => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "Are you sure you want to unblock this user?",
			cont: async (confirmed) => {
				if (confirmed) {
					await api.client.http.DELETE("/api/v1/user/@self/block/{target_id}", {
						params: { path: { target_id: userId } },
					});
					refetch();
				}
			},
		});
	};

	return (
		<>
			<h2>Blocked Users</h2>
			<Show when={blockedUsers.loading}>
				<div>Loading...</div>
			</Show>
			<Show when={blockedUsers.error}>
				<div>Error loading blocked users: {blockedUsers.error.message}</div>
			</Show>
			<Show when={blockedUsers() && blockedUsers()!.items.length === 0}>
				<div>You haven't blocked anyone.</div>
			</Show>
			<ul class="blocked-users-list">
				<For each={blockedUsers()?.items}>
					{(relationship) => (
						<BlockedUserEntry
							relationship={relationship}
							onUnblock={unblockUser}
						/>
					)}
				</For>
			</ul>
		</>
	);
}
