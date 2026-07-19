import { useNavigate } from "@solidjs/router";
import {
	createResource,
	createSignal,
	For,
	Match,
	Show,
	Switch,
} from "solid-js";
import { useApi, useRelationships, useUsers } from "@/api";
import icCheck from "@/assets/check-1.png";
import icDm from "@/assets/dm.png";
import icMore from "@/assets/more.png";
import icX from "@/assets/x-1.png";
import { Icon } from "@/atoms/Icon";
import { Search } from "@/atoms/Search";
import { createTooltip } from "@/atoms/Tooltip";
import { useMenu } from "@/contexts/mod.tsx";
import { AvatarWithStatus } from "./User";

type FilterType = "all" | "online" | "incoming" | "outgoing";

export const Friends = () => {
	const api2 = useApi();
	const relationships = useRelationships();
	const users2 = useUsers();
	const [filter, setFilter] = createSignal<FilterType>("all");

	// TODO: add relationship service

	const [friends] = createResource(async () => {
		const { data } = await api2.client.http.GET("/api/v1/user/@self/friend", {
			params: { query: {} },
		});
		return data;
	});

	const [pending] = createResource(async () => {
		const { data } = await api2.client.http.GET(
			"/api/v1/user/@self/friend/pending",
			{ params: { query: {} } },
		);
		return data;
	});

	const sendRequest = () => {
		const target_id = prompt("target_id");
		if (!target_id) return;
		relationships.send(target_id);
	};

	const filteredFriends = () => {
		const items = [...(friends()?.items ?? []), ...(pending()?.items ?? [])];
		const currentFilter = filter();

		if (currentFilter === "incoming") {
			return items.filter((i) => i.relation === "Incoming");
		} else if (currentFilter === "outgoing") {
			return items.filter((i) => i.relation === "Outgoing");
		} else if (currentFilter === "online") {
			return items.filter((i) => {
				if (i.relation !== "Friend") return false;
				const user = users2.cache.get(i.user_id);
				return user?.presence?.status !== "Offline";
			});
		} else if (currentFilter === "all") {
			return items.filter((i) => i.relation === "Friend");
		}

		return items;
	};

	return (
		<div class="friends">
			<header>
				<h1>friends</h1>
			</header>
			<main>
				<div class="friends-filters">
					<Search placeholder="search" />
					<div class="filters">
						<button
							type="button"
							class="button"
							classList={{ active: filter() === "online" }}
							onClick={() => setFilter("online")}
						>
							online
						</button>
						<button
							type="button"
							class="button"
							classList={{ active: filter() === "all" }}
							onClick={() => setFilter("all")}
						>
							all
						</button>
						<button
							type="button"
							class="button"
							classList={{ active: filter() === "incoming" }}
							onClick={() => setFilter("incoming")}
						>
							incoming
						</button>
						<button
							type="button"
							class="button"
							classList={{ active: filter() === "outgoing" }}
							onClick={() => setFilter("outgoing")}
						>
							outgoing
						</button>
					</div>
					<button type="button" class="button primary" onClick={sendRequest}>
						add friend
					</button>
				</div>
				<ul>
					<For each={filteredFriends()}>
						{(i) => (
							<li>
								<Friend user_id={i.user_id} relation={i.relation} />
							</li>
						)}
					</For>
				</ul>
			</main>
		</div>
	);
};

const Friend = (props: {
	user_id: string;
	relation: string | null | undefined;
}) => {
	const api2 = useApi();
	const relationships = useRelationships();
	const users2 = useUsers();
	const navigate = useNavigate();
	const { setMenu } = useMenu();
	const user = users2.use(() => props.user_id);
	const acceptTooltip = createTooltip({ tip: () => "Accept" });
	const rejectTooltip = createTooltip({ tip: () => "Reject" });
	const cancelTooltip = createTooltip({ tip: () => "Cancel" });
	const dmTooltip = createTooltip({ tip: () => "DM" });
	const moreTooltip = createTooltip({ tip: () => "More" });

	const openDm = async () => {
		const { data } = await api2.client.http.POST(
			"/api/v1/user/@self/dm/{target_id}",
			{ params: { path: { target_id: props.user_id } } },
		);
		if (data && "id" in data) {
			navigate(`/channel/${(data as { id: string }).id}`);
		}
	};

	const acceptRequest = async (e: MouseEvent) => {
		e.stopPropagation();
		await relationships.accept(props.user_id);
		// TODO: refresh friend list
	};

	const rejectRequest = async (e: MouseEvent) => {
		e.stopPropagation();
		await relationships.reject(props.user_id);
		// TODO: refresh friend list
	};

	const openMore = (e: MouseEvent) => {
		e.stopPropagation();
		setMenu({
			type: "user",
			user_id: props.user_id,
			x: e.clientX,
			y: e.clientY,
			admin: false,
		});
	};

	const handleDmClick = (e: MouseEvent) => {
		e.stopPropagation();
		openDm();
	};

	return (
		<div
			class="friend menu-user"
			data-user-id={props.user_id}
			onClick={openDm}
			onKeyDown={(e) => e.key === "Enter" && openDm()}
		>
			<AvatarWithStatus user={user()} />
			<div style="flex:1">
				<div>{user()?.name}</div>
				<Show
					when={
						// TODO: refactor into more robust function
						user()?.presence.activities.find((a) => a.type === "Custom")?.text
					}
				>
					{(t) => <div class="dim">{t()}</div>}
				</Show>
			</div>
			<menu>
				<Switch>
					<Match when={props.relation === "Incoming"}>
						<button
							class="round accept"
							ref={acceptTooltip.content}
							onClick={acceptRequest}
						>
							<Icon src={icCheck} alt="accept friend request" color={null} />
						</button>
						<button
							class="round reject"
							ref={rejectTooltip.content}
							onClick={rejectRequest}
						>
							<Icon src={icX} alt="reject friend request" color={null} />
						</button>
					</Match>
					<Match when={props.relation === "Outgoing"}>
						<button
							class="round reject"
							ref={cancelTooltip.content}
							onClick={rejectRequest}
						>
							<Icon src={icX} alt="cancel friend request" color={null} />
						</button>
					</Match>
					<Match when={props.relation === "Friend"}>
						<button
							class="round"
							ref={dmTooltip.content}
							onClick={handleDmClick}
						>
							<Icon src={icDm} alt="open dm" />
						</button>
					</Match>
				</Switch>
				<button class="round" ref={moreTooltip.content} onClick={openMore}>
					<Icon src={icMore} alt="more..." />
				</button>
			</menu>
		</div>
	);
};
