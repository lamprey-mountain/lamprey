import { createSignal, For, Show } from "solid-js";
import { useApi } from "../api.tsx";
import { Avatar } from "../User.tsx";
import { Time } from "../Time.tsx";
import { createIntersectionObserver } from "@solid-primitives/intersection-observer";
import { useCtx } from "../context.ts";
import { UserAdminMenu } from "../menu/UserAdmin.tsx";
import { UserMenu } from "../menu/User.tsx";

export function Users() {
	const ctx = useCtx();
	const api = useApi();
	const users = api.users.list();

	const fetchMore = () => {
		if (users()?.has_more) {
			api.users.list({ from: users()?.cursor });
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
			<h2>Users</h2>
			<header>
				<div class="name">name</div>
				<div class="joined">registered</div>
			</header>
			<Show when={users()}>
				<ul>
					<For each={users()!.items}>
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
											ctx.setMenu({
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
