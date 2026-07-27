import { createMemo, createSignal, onCleanup, Show } from "solid-js";
import { useApi } from "@/api";
import type { ChannelT } from "@/types";
import { DocumentProvider, useDocument } from "./context";
import { DocumentAside } from "./DocumentAside";
import { DocumentEditor } from "./DocumentEditor";
import { DocumentHeader } from "./DocumentHeader";
import type { ChangesetSelection } from "./types";

// TODO: skeleton ui while document loads

export type DocumentProps = {
	channel: ChannelT;
};

export const Document = (props: DocumentProps) => {
	const api = useApi();
	const doc = useDocument();

	// TODO: finish implementing

	// TODO: mode: 'edit' | 'diff_preview' | 'diff_readonly'

	// TODO: allow changing branch id
	const editContext = createMemo(() => {
		return {
			channelId: props.channel.id,
			branchId: doc.branchId(),
		};
	});

	// const doc = useDocument();

	onCleanup(() => {
		// unsubscribe from document
		// NOTE: this unsubscribes from all documents globally. this isnt a problem
		// right now, but it could be (eg. multiple open documents, lazy unsubscribing
		// instead of resubscribing every route change, etc...)
		api.client.send({
			type: "Subscribe",
			documents: [],
		});
	});

	const disabled = () => false;
	// const mode = () => ...;
	const placeholder = () => "empty document";
	// placeholder={
	// 	mode() === "edit"
	// 		? "write something cool..."
	// 		: mode() === "diff_readonly"
	// 			? "viewing historical revision (readonly)"
	// 			: ""
	// }

	// doc.commands.on("selectChangeset", ...);
	// doc.commands.on("hoverChangeset", ...);

	return (
		<>
			<DocumentHeader channel={props.channel} />
			<div class="document-wrapper">
				<DocumentAside />
				<main class="document-main">
					<Show when={editContext()} keyed>
						{(ctx) => (
							<DocumentEditor
								channelId={ctx.channelId}
								branchId={ctx.branchId}
								disabled={disabled()}
								placeholder={placeholder()}
							/>
						)}
					</Show>
					<Show when={false} keyed>
						{(ctx) => "TODO: diff view?"}
					</Show>
				</main>
			</div>
		</>
	);
};
