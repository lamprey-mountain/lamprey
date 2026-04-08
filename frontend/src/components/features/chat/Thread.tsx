import type { Channel } from "sdk";
import { MemberList } from "@/components/shared/MemberList";

export const ThreadMembers = (props: { thread: Channel }) => {
	return (
		<MemberList
			type="thread"
			id={props.thread.id}
			roomId={props.thread.room_id}
			threadId={props.thread.id}
		/>
	);
};
