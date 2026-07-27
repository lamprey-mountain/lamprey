import { For, Show } from "solid-js";
import { Icon } from "@/atoms/Icon";
import { icChevron } from "@/utils/icons";
import { useDocument } from "./context";

export const DocumentAside = (props: {}) => {
	const doc = useDocument();

	// TODO: button to toggle aside, always hide on small screens, move document-revert to DocumentHistory if aside is hidden
	const isShown = () => true;

	// TODO: hide document-revert if we aren't currently viewing an older revision

	return (
		<Show when={isShown()}>
			<aside class="document-aside">
				<div class="document-revert">
					{/* TODO: port this from old code: viewing revision, restore, cancel; restore menu */}
					<h3>Viewing revision</h3>
					<menu class="actions">
						<button type="button" class="button link">
							Cancel
						</button>
						<button type="button" class="button secondary">
							Restore <Icon class="chevron" src={icChevron} />
						</button>
					</menu>
				</div>
				<Show when={doc.headings().length}>
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
