import { For, Show } from "solid-js";
import { useApi } from "@/api";
import { createTooltip } from "@/atoms/Tooltip";
import type { ChannelT } from "@/types";

export type ThreadTagsProps = {
	thread: ChannelT;
};

export const ThreadTags = (props: ThreadTagsProps) => {
	const api = useApi();

	return (
		<ul class="thread-tags">
			<For each={props.thread.tags ?? []}>
				{(tagId) => {
					const tag = api.tags.useTag(
						() => props.thread.id,
						() => tagId,
					);
					const tip = createTooltip({
						tip: () => (
							<div>
								<Show when={tag()} fallback="loading...">
									{(tag) => (
										<div style="text-align:left">
											<div>{tag().name}</div>
											<Show when={tag().description}>
												{(desc) => <div class="dim">{desc()}</div>}
											</Show>
											<Show when={false /* TODO: ui design for this */}>
												<div class="dim">
													{tag().archived && "(archived)"}
													{tag().restricted && "(restricted)"}
												</div>
											</Show>
										</div>
									)}
								</Show>
							</div>
						),
					});

					return (
						<li
							class="thread-tag"
							classList={{ colored: !!tag()?.color }}
							style={{ "--color": tag()?.color ?? undefined }}
							data-tag-id={tagId}
							ref={tip.content}
							onClick={(e) => {
								e.stopPropagation();
								// TODO: do something here, probably filter threads by tag
							}}
						>
							{tag()?.name ?? "?????"}
						</li>
					);
				}}
			</For>
		</ul>
	);
};
