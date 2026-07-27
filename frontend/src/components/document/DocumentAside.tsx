import { For, Show } from "solid-js";
import { Icon } from "@/atoms/Icon";
import { useChannel } from "@/contexts/mod";
import { icChevron } from "@/utils/icons";
import { useDocument } from "./context";

export const DocumentAside = () => {
	const doc = useDocument();
	const [_chan, updateChan] = useChannel();

	const isRevertShown = () => doc.selectedSeq() || doc.hoverSeq();
	const isTocShown = () => doc.headings().length;
	const isShown = () => (isRevertShown() || isTocShown()) && doc.tocOpen();

	// TODO: always hide DocumentAside on small screens
	// TODO: render document-revert in DocumentHistory if DocumentAside is hidden

	return (
		<Show when={isShown()}>
			<aside class="document-aside">
				<Show when={isRevertShown()}>
					<div class="document-revert">
						<h3>Viewing revision</h3>
						<menu class="actions">
							<button
								type="button"
								class="button link"
								onClick={() => {
									updateChan("history_view", false);
									doc.setSelectedSeq(null);
									doc.setHoverSeq(null);
								}}
							>
								Cancel
							</button>
							<button
								type="button"
								class="button secondary"
								onClick={() => {
									/* TODO: port from old code */
								}}
							>
								Restore <Icon class="chevron" src={icChevron} />
							</button>
						</menu>
					</div>
				</Show>
				<Show when={isTocShown()}>
					<div class="document-toc">
						<h4 class="dim label">Table of Contents</h4>
						<ul>
							<For each={doc.headings()}>
								{(heading) => (
									<li
										class="heading"
										style={{
											"--level": heading.level,
										}}
										// TODO: scroll to heading on click
										// start offset: heading.span.start
										// NOTE: i need to map positions (see frontend/src/components/features/editor/markdown-highlight-plugin.ts)
										// buildTextAndSegments and toDocPos
										// onClick={() => scrollToHeading(heading.span.start)}
									>
										<Show
											when={heading.text.trim()}
											fallback={<em class="untitled">untitled</em>}
										>
											{heading.text}
										</Show>
									</li>
								)}
							</For>
						</ul>
					</div>
				</Show>
			</aside>
		</Show>
	);
};
