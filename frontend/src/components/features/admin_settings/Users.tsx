import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { createMemo, createSignal, For, Show } from "solid-js";
import { useUsers2 } from "@/api";
import { Time } from "../../../atoms/Time.tsx";
import { useCtx } from "../../../context.ts";
import { useMenu } from "../../../contexts/mod.tsx";
import { UserMenu } from "../../../menus/User.tsx";
import { UserAdminMenu } from "../../../menus/UserAdmin.tsx";
import { Avatar } from "../../../User.tsx";

export function Users() {
	const ctx = useCtx();
	const { setMenu } = useMenu();
	const users2 = useUsers2();
	const users = createMemo(() => [...users2.cache.values()]);

	const fetchMore = () => {
		// Users are loaded from cache, no pagination needed
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
			<h2>Users</h2>
			<header>
				<div class="name">name</div>
				<div class="joined">registered</div>
			</header>
			<Show when={users().length > 0}>
				<ul>
					<For each={users()}>
						{(user) => (
							<li>
								<div class="profile">
									<Avatar user={user} />
									<div>
										<h3 class="name">{user.name}</h3>
										<div class="dim">{user.id}</div>
									</div>
								</div>
								<div class="joined">
									<Show when={user.registered_at}>
										<Time date={new Date(user.registered_at!)} />
									</Show>
								</div>
								<div style="flex:1"></div>
								<button
									onClick={(e) => {
										queueMicrotask(() => {
											setMenu({
												type: "user",
												user_id: user.id,
												x: e.clientX,
												y: e.clientY,
												admin: true,
											});
										});
									}}
								>
									options
								</button>
							</li>
						)}
					</For>
				</ul>
				<div ref={setBottom}></div>
			</Show>
		</div>
	);
}
