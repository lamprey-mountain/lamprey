import { createResource, Show } from "solid-js";

// wrapper to lazy load the actual code editor
export const LazyCodeEditor = (props: { source?: string; loading?: boolean }) => {
	const [real] = createResource(async () => {
		const { CodeEditor } = await import("./ScriptEditorInner");
		return CodeEditor;
	});

	return <Show when={real()}>{(component) => component()(props)}</Show>;
};
