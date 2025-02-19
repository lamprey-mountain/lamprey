import { For } from "solid-js";
import { Thread } from "sdk";
import { useApi } from "./api.tsx";

export const ThreadMembers = (props: { thread: Thread }) => {
	const api = useApi();
	const thread_id = () => props.thread.id;

	const members = api.thread_members.list(thread_id);

	return (
		<ul class="room-members">
			<For each={members()?.items}>
				{(i) => {
					const user = api.users.fetch(() => i.user_id);
					const room_member = api.room_members.fetch(
						() => props.thread!.room_id,
						() => i.user_id,
					);
					const thread_member = api.thread_members.fetch(
						() => props.thread.id,
						() => i.user_id,
					);

					function name() {
						let name: string | undefined | null = null;
						const tm = thread_member();
						if (tm?.membership === "Join") name ??= tm.override_name;

						const rm = room_member?.();
						if (rm?.membership === "Join") name ??= rm.override_name;

						name ??= user()?.name;
						return name;
					}

					return <li data-user-id={i.user_id}>{name()}</li>;
				}}
			</For>
		</ul>
	);
};
