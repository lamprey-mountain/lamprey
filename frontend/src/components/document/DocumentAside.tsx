import { Show } from "solid-js";

export const DocumentAside = (props: {}) => {
	// TODO: rename .document-left-rail -> .document-aside, extract into component
	return (
		<Show when={false}>
			<aside class="document-left-rail">
				{/* TODO: viewing revision, restore, cancel; restore menu */}
				{/* TODO: table of contents */}
			</aside>
		</Show>
	);
};
