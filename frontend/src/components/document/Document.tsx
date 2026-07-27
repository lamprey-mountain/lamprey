import { createMemo, Show } from "solid-js";
import type { ChannelT } from "@/types";
import { DocumentProvider } from "./context";
import { DocumentAside } from "./DocumentAside";
import { DocumentEditor } from "./DocumentEditor";
import { DocumentHeader } from "./DocumentHeader";

export type DocumentProps = {
	channel: ChannelT;
};

export const Document = (props: DocumentProps) => {
	// TODO: finish implementing

	// TODO: allow changing branch id
	const editContext = createMemo(() => {
		return {
			channelId: props.channel.id,
			branchId: props.channel.id,
		};
	});

	return (
		<DocumentProvider>
			<DocumentHeader channel={props.channel} />
			<div class="document-wrapper">
				<DocumentAside />
				<main class="document-main">
					<Show when={editContext()} keyed>
						{(ctx) => (
							<DocumentEditor
								channelId={ctx.channelId}
								branchId={ctx.branchId}
								disabled={false}
								// TODO: impl diff view
								// placeholder={
								// 	mode() === "edit"
								// 		? "write something cool..."
								// 		: mode() === "diff_readonly"
								// 			? "viewing historical revision (readonly)"
								// 			: ""
								// }
								placeholder="empty document"
							/>
						)}
					</Show>
				</main>
			</div>
		</DocumentProvider>
	);
};
