import type { Channel } from "sdk";
import { createSignal } from "solid-js";
import { createEditor } from "./DocumentEditor.tsx";

type DocumentProps = {
	channel: Channel,
}

export const Document = (props: DocumentProps) => {
	const [branchId, setBranchId] = createSignal(props.channel.id);
	// setup ydoc here, pass to DocumentMain?

	return (
		<div class="document">
			<DocumentHeader channel={props.channel} />
			<DocumentMain channel={props.channel} />
		</div>
	);
};

const DocumentHeader = (props: DocumentProps) => {
	// top: title, topic(?), notifications, members, search
	// bottom: branches (merge, diff), edit, format, insert, view, tools
	return "todo";
};

const DocumentMain = (props: DocumentProps) => {
	const editor = createEditor({}, props.channel.id, props.channel.id);

	return (
		<main>
			<editor.View
				onSubmit={() => false}
				channelId={props.channel.id}
			/>
		</main>
	);
};

export const Wiki = (props: DocumentProps) => {
	// maybe copy forum channels here
	return "todo";
};
