import { For, Show } from "solid-js";
import { useCtx } from "./context";

export const MessageSkeleton = () => {
	const ctx = useCtx();
	const messageStyle = ctx.userConfig().frontend["message_style"] || "cozy";
	const isCozy = messageStyle === "cozy";

	const skeletonItems = Array.from({ length: 20 }, (_, i) => ({
		id: `skeleton-${i}`,
		hasAvatar: isCozy,
		hasAttachment: Math.random() > .7,
		contentLines: Array.from({
			length: Math.floor(Math.random() * (isCozy ? 8 : 5) + (isCozy ? 4 : 3)),
		}, () => Math.random() * 40 + 10),
	}));

	return (
		<For each={skeletonItems}>
			{(item) => (
				<li
					class="message skeleton-message"
					classList={{ withavatar: item.hasAvatar }}
				>
					<Show when={item.hasAvatar}>
						<div class="avatar-wrap">
							<div class="avatar skeleton-avatar"></div>
						</div>
						<div class="author">
							<div class="skeleton-name"></div>
							<div class="skeleton-time"></div>
						</div>
					</Show>
					<Show when={!item.hasAvatar}>
						<div class="author-wrap">
							<div class="author sticky">
								<div class="skeleton-name"></div>
							</div>
						</div>
						<div class="skeleton-time-compact"></div>
					</Show>
					<div class="content">
						<div class="body">
							<For each={item.contentLines}>
								{(w) => <div class="skeleton-line" style={`width:${w}%`}></div>}
							</For>
						</div>
						<Show when={item.hasAttachment}>
							<ul class="attachments">
								<li class="skeleton-attachment raw"></li>
							</ul>
						</Show>
					</div>
				</li>
			)}
		</For>
	);
};
