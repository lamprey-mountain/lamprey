import { useApi } from "../api.tsx";
import { useCtx } from "../context.ts";
import { Item, Menu, Separator } from "./Parts.tsx";

type ThreadMemberMenuProps = {
	thread_id: string;
	user_id: string;
};

export function ThreadMemberMenu(props: ThreadMemberMenuProps) {
	const ctx = useCtx();
	const api = useApi();
	const member = api.thread_members.fetch(
		() => props.thread_id,
		() => props.user_id,
	);
	const thread = api.threads.fetch(
		() => props.thread_id,
	);

	const copyUserId = () => navigator.clipboard.writeText(props.user_id);

	const logToConsole = () => console.log(JSON.parse(JSON.stringify(member())));

	const kickThread = () => {
		ctx.dispatch({
			do: "modal.confirm",
			text: "really kick?",
			cont: (conf) => {
				if (!conf) return;
				api.client.http.DELETE("/api/v1/thread/{thread_id}/member/{user_id}", {
					params: {
						path: {
							thread_id: props.thread_id,
							user_id: props.user_id,
						},
					},
				});
			},
		});
	};

	const kickRoom = () => {
		const room_id = thread()?.room_id;
		if (!room_id) return;
		ctx.dispatch({
			do: "modal.confirm",
			text: "really kick?",
			cont: (conf) => {
				if (!conf) return;
				api.client.http.DELETE("/api/v1/room/{room_id}/member/{user_id}", {
					params: {
						path: {
							room_id,
							user_id: props.user_id,
						},
					},
				});
			},
		});
	};

	return (
		<Menu>
			<Item>todo block</Item>
			<Item>todo dm</Item>
			<Separator />
			<Item onClick={kickThread}>kick (thread)</Item>
			<Item onClick={kickRoom}>kick (room)</Item>
			<Separator />
			<Item onClick={copyUserId}>copy user id</Item>
			<Item onClick={logToConsole}>log to console</Item>
		</Menu>
	);
}
