import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { createSignal, For, Show, type VoidProps } from "solid-js";
import { useApi, useRoomBans, useUsers } from "@/api";
import { useCtx } from "@/app/context";
import { Search } from "@/atoms/Search";
import { Time } from "@/atoms/Time.tsx";
import { createTooltip } from "@/atoms/Tooltip.tsx";
import { Avatar } from "@/components/shared/User";
import type { RoomT } from "@/types";
import { getDate } from "@/utils/general";

export function Bans(props: VoidProps<{ room: RoomT }>) {
	const _ctx = useCtx();
	const api2 = useApi();
	const roomBans2 = useRoomBans();
	const users2 = useUsers();
	const bans = roomBans2.useList(() => props.room.id);

	const [bottom, setBottom] = createSignal<Element | undefined>();
	const [search, setSearch] = createSignal("");

	const filteredIds = () => {
		const ids = bans()?.state.ids ?? [];
		const query = search().trim().toLowerCase();
		if (!query) return ids;
		return ids.filter((id) => {
			const ban = roomBans2.cache.get(id);
			if (!ban) return false;
			const name = users2.cache.get(ban.user_id)?.name ?? ban.user_id;
			return (
				name.toLowerCase().includes(query) ||
				(ban.reason ?? "").toLowerCase().includes(query)
			);
		});
	};

	createIntersectionObserver(
		() => (bottom() ? [bottom()!] : []),
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting && bans()?.state.has_more) {
					// Trigger refetch for pagination (TODO: implement proper pagination)
					// bans()?.refetch?.();
				}
			}
		},
	);

	const unban = (user_id: string) => {
		api2.client.http.DELETE("/api/v1/room/{room_id}/ban/{user_id}", {
			params: { path: { room_id: props.room.id, user_id } },
		});
	};

	return (
		<div class="room-settings-bans">
			<h2>bans</h2>
			<Search
				class="bans-search"
				placeholder="search bans..."
				onInput={setSearch}
			/>
			<header>
				<div class="name">name</div>
				<div class="created">created at</div>
				<div class="expires">expires at</div>
				<div class="reason">reason</div>
			</header>
			<Show when={bans()}>
				<ul>
					<For each={filteredIds()}>
						{(id) => {
							const ban = roomBans2.cache.get(id);
							if (!ban) return null;
							const user = users2.use(() => ban.user_id);
							const name = () => user()?.name;
							const { content: tipContent } = createTooltip({
								tip: () => ban.reason,
							});
							return (
								<li>
									<div class="profile">
										<Avatar user={user()} />
										<div>
											<h3 class="name">{name()}</h3>
										</div>
									</div>
									<div class="created">
										<Time date={getDate(ban.created_at)} />
									</div>
									<div class="expires">
										<Show when={ban.expires_at}>
											{(exp) => <Time date={getDate(exp())} />}
										</Show>
									</div>
									<div class="reason">{ban.reason}</div>
									<button
										type="button"
										class="button"
										onClick={[unban, ban.user_id]}
									>
										unban
									</button>
								</li>
							);
						}}
					</For>
				</ul>
				<div ref={setBottom}></div>
			</Show>
		</div>
	);
}
