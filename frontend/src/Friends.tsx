import { createResource, For, Show } from "solid-js";
import { useApi } from "./api";
import { AvatarWithStatus } from "./User";

export const Friends = () => {
	const api = useApi();

	const [friends] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/friend",
			{ params: { path: { user_id: "@self" } } },
		);
		return data;
	});

	const sendRequest = () => {
		const target_id = prompt("target_id");
		if (!target_id) return;
		api.client.http.PUT("/api/v1/user/@self/friend/{target_id}", {
			params: { path: { target_id } },
		});
	};

	return (
		<div class="friends" style="padding:8px">
			<h1>friends</h1>
			<div class="info">
				{/* TODO; add search icon here */}
				<input type="search" placeholder="search" />
				<div class="filter">
					<button>online</button>
					<button>all</button>
					<button>pending</button>
				</div>
				<button class="primary" onClick={sendRequest}>add</button>
			</div>
			<ul>
				<For each={friends()?.items}>
					{(i) => (
						<li>
							<Friend user_id={i.user_id} />
						</li>
					)}
				</For>
			</ul>
		</div>
	);
};

const Friend = (props: { user_id: string }) => {
	const api = useApi();
	const user = api.users.fetch(() => props.user_id);

	return (
		<div class="friend menu-user" data-user-id={props.user_id}>
			<AvatarWithStatus user={user()} />
			<div>
				<div>{user()?.name}</div>
				<Show
					when={user()?.presence.activities.find((a) => a.type === "Custom")
						?.text}
				>
					{(t) => <div class="dim">{t()}</div>}
				</Show>
			</div>
		</div>
	);
};
