import type { EditorState } from "prosemirror-state";
import { useAutocomplete } from "@/contexts/autocomplete";
import { useFormattingToolbar } from "@/contexts/formatting-toolbar";
import { createEditor } from "../editor/Editor";
import { serializeToMarkdown } from "../editor/serializer";
import { type ApplicationDraft, useApplications } from "./context";

export const BridgeSettings = (props: { draft: ApplicationDraft }) => {
	const apps = useApplications();
	const toolbar = useFormattingToolbar();
	const autocomplete = useAutocomplete();

	// TODO: move to utils file?
	const getId = (draft: ApplicationDraft) => {
		if (draft.state === "create") return draft.nonce;
		return draft.data.id;
	};

	const id = () => getId(props.draft);

	const currentData = () => {
		const d = props.draft;
		if (d.state === "create") return d.create;
		if (d.state === "update") return { ...d.data, ...d.update };
		return d.data;
	};

	const bridge = () => {
		return currentData().bridge ?? {};
	};

	const descriptionEditor = createEditor({
		channelId: () => id(),
		autocomplete,
		toolbar,
		initialContent: () => bridge().platform_description ?? "",
	});

	// TODO: make typescript happy
	const updateBridge = (key: string, value: any) => {
		apps.updateDraft(id(), {
			bridge: {
				...bridge(),
				[key]: value,
			},
		});
	};

	const handleDescriptionChange = (state: EditorState) => {
		const s = serializeToMarkdown(state.doc);
		updateBridge("platform_description", s || null);
	};

	return (
		<div class="bridge-details">
			<div>{/* spacer */}</div>
			<h3 class="top">bridge metadata</h3>
			<label>
				<h3 class="dim">platform name</h3>
				<input
					type="text"
					placeholder="Other platform"
					value={bridge().platform_name ?? ""}
					onInput={(e) => {
						updateBridge("platform_name", e.currentTarget.value || null);
					}}
				/>
			</label>
			<label>
				<h3 class="dim">platform url</h3>
				<input
					type="text"
					placeholder="https://example.com"
					value={bridge().platform_url ?? ""}
					onInput={(e) => {
						updateBridge("platform_url", e.currentTarget.value || null);
					}}
				/>
			</label>
			<label>
				<h3 class="dim">platform description</h3>
				<descriptionEditor.View
					placeholder="platform description..."
					submitOnEnter={false}
					onChange={handleDescriptionChange}
					channelId={id()}
					autofocus={false}
				/>
			</label>
		</div>
	);
};
