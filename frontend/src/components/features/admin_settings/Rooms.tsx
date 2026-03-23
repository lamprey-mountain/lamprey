import { createSignal, For, Show } from "solid-js";
import { useApi, useRooms2 } from "../../../api.tsx";
import { Avatar } from "../../../User.tsx";
import { Time } from "../../../atoms/Time.tsx";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { getThumbFromId } from "../../../media/util.tsx";
import { getTimestampFromUUID } from "sdk";

export function Rooms() {
	const api = useApi();
	const api2 = useRooms2();
	const rooms = api2.useListAll();

	const fetchMore = () => {
		if (rooms.has_more) {
			api2.fetchListAll(rooms.cursor);
		}
	};

	const [bottom, setBottom] = createSignal<Element | undefined>();

	createIntersectionObserver(
		() => (bottom() ? [bottom()!] : []),
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting) fetchMore();
			}
		},
	);

	return (
		<div class="room-settings-members">
			<h2>Rooms</h2>
			<header>
				<div class="name">name</div>
				<div class="joined">created</div>
			</header>
			<Show when={rooms.ids.length > 0}>
				<ul>
					<For each={rooms.ids}>
						{(id) => {
							const room = api2.get(id);
							if (!room) return null;
							return (
								<li>
									<div class="profile">
										<Show
											when={room.icon}
											fallback={<div class="avatar">{room.name}</div>}
										>
											<img
												src={getThumbFromId(room.icon!, 64)}
												class="avatar"
											/>
										</Show>
										<div>
											<h3 class="name">{room.name}</h3>
											<div class="dim">{room.id}</div>
										</div>
									</div>
									<div class="joined">
										<Time date={getTimestampFromUUID(room.id)} />
									</div>
									<div style="flex:1"></div>
									<button>options</button>
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
