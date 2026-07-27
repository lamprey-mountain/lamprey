import { createMemo, Match, onCleanup, Switch } from "solid-js";
import { useApi } from "@/api";
import type { ChannelT } from "@/types";
import { useDocument } from "./context";
import { DocumentAside } from "./DocumentAside";
import { DocumentDiffView } from "./DocumentDiffView";
import { DocumentEditor } from "./DocumentEditor";
import { DocumentHeader } from "./DocumentHeader";

// TODO: skeleton ui while document loads

export type DocumentProps = {
	channel: ChannelT;
};

export const Document = (props: DocumentProps) => {
	const api = useApi();
	const doc = useDocument();

	const editContext = createMemo(() => {
		return {
			channelId: props.channel.id,
			branchId: doc.branchId(),
		};
	});

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

	// TODO: move this into document context?
	const activeChangeset = createMemo(() => {
		const hover = doc.hoverSeq();
		const selected = doc.selectedSeq();

		if (hover && selected) {
			return {
				start_seq: Math.min(hover.start_seq, selected.start_seq),
				end_seq: Math.max(hover.end_seq, selected.end_seq),
			};
		}
		return hover ?? selected;
	});

	// maybe remove mode()?
	const mode = createMemo(() => (activeChangeset() ? "diff" : "edit"));
	const disabled = () => mode() === "diff";
	const placeholder = () =>
		mode() === "edit" ? "write something cool..." : "revision has no content";

	// doc.commands.on("selectChangeset", ...);
	// doc.commands.on("hoverChangeset", ...);

	return (
		<>
			<DocumentHeader channel={props.channel} />
			<div class="document-wrapper">
				<DocumentAside />
				<main class="document-main">
					<Switch>
						<Match when={activeChangeset()}>
							{(c) => (
								<DocumentDiffView
									channelId={props.channel.id}
									changeset={c()}
									placeholder={placeholder()}
								/>
							)}
						</Match>
						<Match when={editContext()} keyed>
							{(ctx) => (
								<DocumentEditor
									channelId={ctx.channelId}
									branchId={ctx.branchId}
									disabled={disabled()}
									placeholder={placeholder()}
								/>
							)}
						</Match>
					</Switch>
				</main>
			</div>
		</>
	);
};
