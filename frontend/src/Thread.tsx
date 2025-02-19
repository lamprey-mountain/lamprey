import { For, Show } from "solid-js";
import { Thread } from "sdk";
import { useApi } from "./api.tsx";
import { tooltip } from "./Tooltip.tsx";
import { UserView } from "./User.tsx";

export const ThreadMembers = (props: { thread: Thread }) => {
	const api = useApi();
	const thread_id = () => props.thread.id;
	const room_id = () => props.thread.room_id;
	const members = api.thread_members.list(thread_id);

	return (
		<ul class="member-list" data-thread-id={props.thread.id}>
			<For each={members()?.items}>
				{(i) => {
					const user_id = () => i.user_id;
					const user = api.users.fetch(user_id);
					const room_member = api.room_members.fetch(room_id, user_id);
					const thread_member = api.thread_members.fetch(thread_id, user_id);

					function name() {
						let name: string | undefined | null = null;
						const tm = thread_member();
						if (tm?.membership === "Join") name ??= tm.override_name;

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
								thread_member={thread_member()}
							/>
						</Show>,
						<li class="menu-user" data-user-id={i.user_id}>{name()}</li>,
					);
				}}
			</For>
		</ul>
	);
};
