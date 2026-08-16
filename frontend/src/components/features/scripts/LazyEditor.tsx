import type { Script } from "sdk";
import { createResource, Show } from "solid-js";

// wrapper to lazy load the actual code editor
export const LazyCodeEditor = (props: {
	script: Script;
	onChange?: (val: string) => void;
}) => {
	const [real] = createResource(async () => {
		const { CodeEditor } = await import("./Editor");
		return CodeEditor;
	});

	// TODO: use Suspense

	return <Show when={real()}>{(component) => component()(props)}</Show>;
};
