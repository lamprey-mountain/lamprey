import { throttle } from "@solid-primitives/scheduled";
import type { User, UserListFilter } from "sdk";
import { createEffect, createSignal, onMount, Show } from "solid-js";
import { useApi, useUsers } from "@/api";
import { Dropdown } from "@/atoms/Dropdown";
import { Time } from "@/atoms/Time.tsx";
import { Avatar } from "@/components/shared/User";
import { useMenu } from "@/contexts/mod.tsx";
import { useNavigate } from "@/contexts/router";
import { getDate } from "@/utils/general";
import { ResourceList } from "./ResourceList";

const filterOptions: { item: UserListFilter | ""; label: string }[] = [
	{ item: "", label: "all users" },
	{ item: "registered", label: "registered" },
	{ item: "guest", label: "guests" },
	{ item: "bot", label: "bots" },
	{ item: "puppet", label: "puppets" },
];

const UserBadges = (props: { user: User }) => (
	<div class="admin-badges">
		<Show when={props.user.suspended}>
			<span class="admin-badge danger">suspended</span>
		</Show>
		<Show when={props.user.deleted_at}>
			<span class="admin-badge danger">deleted</span>
		</Show>
		<Show when={props.user.bot}>
			<span class="admin-badge">bot</span>
		</Show>
		<Show when={props.user.system}>
			<span class="admin-badge">system</span>
		</Show>
		<Show when={!props.user.registered_at}>
			<span class="admin-badge dim">guest</span>
		</Show>
	</div>
);

export function Users() {
	const { setMenu } = useMenu();
	const nav = useNavigate();
	const api = useApi();
	const users2 = useUsers();

	const [query, setQuery] = createSignal("");
	const [filter, setFilter] = createSignal<UserListFilter | "">("");
	const [searchResults, setSearchResults] = createSignal<User[] | null>(null);
	const [searching, setSearching] = createSignal(false);

	const search = throttle(async (q: string) => {
		if (q.length === 0) {
			setSearchResults(null);
			return;
		}

		setSearching(true);
		try {
			const results = await api.users.search(q);
			setSearchResults(results.users ?? []);
		} catch {
			setSearchResults([]);
		} finally {
			setSearching(false);
		}
	}, 500);

	createEffect(() => {
		search(query());
	});

	const loadPage = () => {
		users2.loadListAll(filter() || undefined).catch(() => {});
	};

	const loadFirstPage = () => {
		users2.resetListAll();
		loadPage();
	};

	onMount(loadFirstPage);

	const selectFilter = (f: UserListFilter | "") => {
		if (f === filter()) return;
		setFilter(f);
		loadFirstPage();
	};

	const pagedUsers = () =>
		users2.userList.state.ids
			.map((id) => users2.cache.get(id))
			.filter((user) => user !== undefined);

	const items = () =>
		query().length > 0 ? (searchResults() ?? []) : pagedUsers();

	const hasMore = () => query().length === 0 && users2.userList.state.has_more;

	const openUser = (user: User) => nav(`/user/${user.id}`);

	return (
		<ResourceList
			title="Users"
			class="room-settings-members"
			query={query}
			onSearch={setQuery}
			placeholder="Search users..."
			autofocus
			filters={
				<Dropdown
					selected={filter()}
					options={filterOptions}
					onSelect={(item) => selectFilter(item ?? "")}
				/>
			}
			header={
				<>
					<div class="name">name</div>
					<div class="joined">registered</div>
					<div style="flex:1">status</div>
				</>
			}
			items={items}
			itemKey={(user) => user.id}
			hasMore={hasMore}
			onLoadMore={loadPage}
			loading={() => searching() || users2.userList.state.isLoading}
			empty={query().length > 0 ? "no users match this search" : "no users"}
			renderItem={(user) => (
				<>
					<button
						type="button"
						class="profile"
						style="text-align:start;background:none;border:none;padding:0;cursor:pointer"
						onClick={() => openUser(user)}
					>
						<Avatar user={user} />
						<div>
							<h3 class="name">{user.name}</h3>
							<div class="dim">{user.id}</div>
						</div>
					</button>
					<div class="joined">
						<Show when={user.registered_at}>
							<Time date={getDate(user.registered_at!)} />
						</Show>
					</div>
					<UserBadges user={user} />
					<div style="flex:1"></div>
					<button
						type="button"
						class="button"
						onClick={(e) => {
							e.stopPropagation();
							setMenu({
								type: "user",
								user_id: user.id,
								x: e.clientX,
								y: e.clientY,
								admin: true,
							});
						}}
					>
						options
					</button>
				</>
			)}
		/>
	);
}
