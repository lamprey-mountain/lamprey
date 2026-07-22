import { useNavigate } from "@solidjs/router";
import { getTimestampFromUUID } from "sdk";
import { Show } from "solid-js";
import { useCtx } from "@/app/context";
import { Markdown } from "@/atoms/Markdown";
import { Time } from "@/atoms/Time";
import { ChannelIcon } from "@/avatar/ChannelIcon";
import { useChannel } from "@/contexts/mod";
import type { ChannelT } from "@/types";

export type ThreadCardProps = {
	thread: ChannelT;
	openInSidebar: boolean;
};

export const ThreadCard = (props: ThreadCardProps) => {
	const nav = useNavigate();
	const [_ch, chUpdate] = useChannel();
	const ctx = useCtx();

	const goto = () => {
		ctx.setThreadsView(null);

		if (props.openInSidebar) {
			chUpdate("thread_chat_sidebar_thread_id", props.thread.id);
		} else {
			nav(`/thread/${props.thread.id}`);
		}
	};

	return (
		<article
			class="thread menu-thread thread-card"
			data-thread-id={props.thread.id}
			onClick={goto}
			onKeyDown={(e) => e.key === "Enter" && goto()}
		>
			<header>
				<header class="top">
					<ChannelIcon channel={props.thread} />
					<div class="spacer">{props.thread.name}</div>
					<div class="time">
						Created <Time date={getTimestampFromUUID(props.thread.id)} />
					</div>
				</header>
				<div class="bottom">
					<div class="dim">
						<Show when={!!props.thread.deleted_at}>
							<span class="removed">removed</span> &bull;{" "}
						</Show>
						{props.thread.message_count} message(s) &bull; last msg{" "}
						<Time
							date={getTimestampFromUUID(
								props.thread.last_version_id ?? props.thread.id,
							)}
						/>
					</div>
					<Show when={props.thread.description}>
						{(desc) => <Markdown content={desc()} class="description" />}
					</Show>
				</div>
			</header>
		</article>
	);
};
