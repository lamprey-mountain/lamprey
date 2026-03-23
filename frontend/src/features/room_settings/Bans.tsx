import {
	createEffect,
	createSignal,
	For,
	onCleanup,
	Show,
	type VoidProps,
} from "solid-js";
import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import type { RoomT } from "../types.ts";
import { Role, RoomMember, RoomMemberOrigin } from "sdk";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { Avatar } from "../User.tsx";
import { Time } from "../Time.tsx";
import { useFloating } from "solid-floating-ui";
import { ReferenceElement, shift } from "@floating-ui/dom";
import { usePermissions } from "../hooks/usePermissions.ts";
import { createTooltip } from "../Tooltip.tsx";

export function Bans(props: VoidProps<{ room: RoomT }>) {
	const ctx = useCtx();
	const api = useApi();
	const bans = api.room_bans.list(() => props.room.id);

	const fetchMore = () => {
		api.room_bans.list(() => props.room.id);
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(() => bottom() ? [bottom()!] : [], (entries) => {
		for (const entry of entries) {
			if (entry.isIntersecting) fetchMore();
		}
	});

	const unban = (user_id: string) => {
		api.client.http.DELETE("/api/v1/room/{room_id}/ban/{user_id}", {
			params: { path: { room_id: props.room.id, user_id } },
		});
	};

	return (
		<div class="room-settings-bans">
			<h2>bans</h2>
			<header>
				<div class="name">name</div>
				<div class="created">created at</div>
				<div class="expires">expires at</div>
				<div class="reason">reason</div>
			</header>
			<Show when={bans()}>
				<ul>
					<For each={bans()!.items}>
						{(i) => {
							const user = api.users.fetch(() => i.user_id);
							const name = () => user()?.name;
							const { content: tipContent } = createTooltip({
								tip: () => i.reason,
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
										<Time date={new Date(i.created_at)} />
									</div>
									<div class="expires">
										<Show when={i.expires_at}>
											{(exp) => <Time date={new Date(exp())} />}
										</Show>
									</div>
									<div class="reason">
										{i.reason}
									</div>
									<button
										onClick={[unban, i.user_id]}
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
