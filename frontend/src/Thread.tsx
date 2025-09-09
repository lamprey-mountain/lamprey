import { For, Show } from "solid-js";
import type { Thread } from "sdk";
import { useApi } from "./api.tsx";
import { tooltip } from "./Tooltip.tsx";
import { AvatarWithStatus, UserView } from "./User.tsx";

export const ThreadMembers = (props: { thread: Thread }) => {
	const api = useApi();
	const thread_id = () => props.thread.id;
	const room_id = () => props.thread.room_id;
	const members = api.thread_members.list(thread_id);

	return (
		<ul class="member-list" data-thread-id={props.thread.id}>
			<For each={members()?.items.filter((m) => m.membership === "Join")}>
				{(member) => {
					const user_id = () => member.user_id;
					const user = api.users.fetch(user_id);
					const room_member = props.thread?.room_id
						? api.room_members.fetch(
							room_id,
							user_id,
						)
						: () => null;

					function name() {
						let name: string | undefined | null = null;

						const rm = room_member();
						if (rm?.membership === "Join") name ??= rm.override_name;

						name ??= user()?.name;
						return name;
					}

					return tooltip(
						{
							placement: "left-start",
						},
						<Show when={user()}>
							<UserView
								user={user()}
								room_member={room_member()}
								thread_member={member}
							/>
						</Show>,
						<li class="menu-user" data-user-id={member.user_id}>
							<AvatarWithStatus user={user()} />
							<span class="text">
								<span class="name">{name()}</span>
								<Show when={false}>
									<span class="status-message">asdf</span>
								</Show>
							</span>
						</li>,
					);
				}}
			</For>
		</ul>
	);
};
