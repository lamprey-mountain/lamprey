import { For, Show } from "solid-js";
import { useCtx } from "@/app/context";

// TODO: add messageGroupLengths, make MessageSkeletons pass `separate` prop/class
export const MessageSkeleton = () => {
	const ctx = useCtx();
	const messageStyle = () => ctx.preferences().frontend.message_style || "cozy";

	// TODO: make this reactive
	const isCozy = messageStyle() === "cozy";

	const attachment =
		Math.random() > 0.7
			? { width: Math.random() * 200 + 100, height: Math.random() * 200 + 100 }
			: null;
	const authorWordLength = Math.random() * 40 + 10;

	const bodyWordLengths = Array.from(
		{
			length: Math.floor(Math.random() * (isCozy ? 8 : 5) + (isCozy ? 4 : 3)),
		},
		() => Math.random() * 40 + 10,
	);

	return (
		<article
			class="message skeleton"
			role="presentation"
			classList={{ separate: Math.random() > 0.7 }}
		>
			<aside class="aside">
				<div class="avatar ghost"></div>
			</aside>

			<div class="content">
				<h3 class="header">
					<div
						class="text ghost author"
						style={`width:${authorWordLength * 3}px`}
					></div>
				</h3>

				<div class="body">
					<For each={bodyWordLengths}>
						{(w) => <div class="text ghost" style={`width:${w * 3}px`}></div>}
					</For>
				</div>
			</div>

			<div class="accessories">
				<Show when={attachment}>
					{(a) => (
						<ul class="attachments">
							<li
								class="attachment raw ghost"
								style={`height:${a().height}px;width:${a().width}px`}
							></li>
						</ul>
					)}
				</Show>
			</div>
		</article>
	);
};

export const MessageSkeletons = () => {
	return (
		<For each={Array.from({ length: 20 }, (_, i) => i)}>
			{() => <MessageSkeleton />}
		</For>
	);
};
