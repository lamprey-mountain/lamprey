import { createResource, createSignal, For, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { useApi } from "./api";
import { AvatarWithStatus } from "./User";
import type { RelationshipType } from "sdk";

type FilterType = "all" | "online" | "incoming" | "outgoing";

export const Friends = () => {
	const api = useApi();
	const navigate = useNavigate();
	const [filter, setFilter] = createSignal<FilterType>("all");

	const [friends] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/friend",
			{ params: { path: { user_id: "@self" } } },
		);
		return data;
	});

	const [pending] = createResource(async () => {
		const { data } = await api.client.http.GET(
			"/api/v1/user/{user_id}/friend/pending",
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

	const filteredFriends = () => {
		const items = friends()?.items ?? [];
		const currentFilter = filter();

		if (currentFilter === "incoming") {
			return items.filter((i) => i.relation === "Incoming");
		} else if (currentFilter === "outgoing") {
			return items.filter((i) => i.relation === "Outgoing");
		} else if (currentFilter === "online") {
			return items.filter((i) => {
				const user = api.users.cache.get(i.user_id);
				return user?.presence?.status !== "Offline";
			});
		}
		return items;
	};

	return (
		<div class="friends" style="padding:8px">
			<h1>friends</h1>
			<div class="info">
				{/* TODO; add search icon here */}
				<input type="search" placeholder="search" />
				<div class="filter">
					<button
						classList={{ active: filter() === "online" }}
						onClick={() => setFilter("online")}
					>
						online
					</button>
					<button
						classList={{ active: filter() === "all" }}
						onClick={() => setFilter("all")}
					>
						all
					</button>
					<button
						classList={{ active: filter() === "incoming" }}
						onClick={() => setFilter("incoming")}
					>
						incoming
					</button>
					<button
						classList={{ active: filter() === "outgoing" }}
						onClick={() => setFilter("outgoing")}
					>
						outgoing
					</button>
				</div>
				<button class="primary" onClick={sendRequest}>add</button>
			</div>
			<ul>
				<For each={filteredFriends()}>
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
	const navigate = useNavigate();
	const user = api.users.fetch(() => props.user_id);

	const openDm = async () => {
		const { data } = await api.client.http.POST(
			"/api/v1/user/@self/dm/{target_id}",
			{ params: { path: { target_id: props.user_id } } },
		);
		if (data) {
			navigate(`/channel/${data.id}`);
		}
	};

	return (
		<div
			class="friend menu-user"
			data-user-id={props.user_id}
			onClick={openDm}
			style="cursor: pointer"
		>
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
