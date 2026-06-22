import { For, Show } from "solid-js";
import { useCtx } from "@/app/context";

export const MessageSkeleton = () => {
	const ctx = useCtx();
	const messageStyle = ctx.preferences().frontend.message_style || "cozy";
	const isCozy = messageStyle === "cozy";

	const hasAvatar = isCozy;
	const hasAttachment = Math.random() > 0.7;
	const contentLines = Array.from(
		{
			length: Math.floor(Math.random() * (isCozy ? 8 : 5) + (isCozy ? 4 : 3)),
		},
		() => Math.random() * 40 + 10,
	);

	return (
		<li class="message skeleton-message" classList={{ withavatar: hasAvatar }}>
			<Show when={hasAvatar}>
				<div class="avatar-wrap">
					<div class="avatar skeleton-avatar"></div>
				</div>
				<div class="author">
					<div class="skeleton-name"></div>
					<div class="skeleton-time"></div>
				</div>
			</Show>
			<Show when={!hasAvatar}>
				<div class="author-wrap">
					<div class="author sticky">
						<div class="skeleton-name"></div>
					</div>
				</div>
				<div class="skeleton-time-compact"></div>
			</Show>
			<div class="content">
				<div class="body">
					<For each={contentLines}>
						{(w) => <div class="skeleton-line" style={`width:${w}%`}></div>}
					</For>
				</div>
				<Show when={hasAttachment}>
					<ul class="attachments">
						<li class="skeleton-attachment raw"></li>
					</ul>
				</Show>
			</div>
		</li>
	);
};

export const MessageSkeletons = () => {
	return (
		<For each={Array.from({ length: 20 }, (_, i) => i)}>
			{() => <MessageSkeleton />}
		</For>
	);
};
