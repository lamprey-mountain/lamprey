import { type Accessor, For, Show } from "solid-js";
import type { TocHeading } from "@/types/chat";

export type DocumentTocPopoutProps = {
	headings: Accessor<TocHeading[]>;
	onSelect: (heading: TocHeading) => void;
};

export const DocumentTocPopout = (props: DocumentTocPopoutProps) => {
	return (
		<div
			class="threads-popout document-toc-popout"
			onClick={(e) => e.stopPropagation()}
		>
			<div class="background"></div>
			<div class="content">
				<div class="header">
					<h3 class="dim">Table of Contents</h3>
				</div>
				<div class="thread-list">
					<Show
						when={props.headings().length > 0}
						fallback={
							<div
								class="dim"
								style="text-align:center;margin:2em;font-size:1rem;"
							>
								no headings
							</div>
						}
					>
						<ul class="document-toc-list">
							<For each={props.headings()}>
								{(heading) => (
									<li
										class="heading"
										style={{
											"--level": heading.level,
										}}
										onClick={() => props.onSelect(heading)}
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
					</Show>
				</div>
			</div>
		</div>
	);
};
