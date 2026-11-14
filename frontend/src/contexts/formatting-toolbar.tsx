import type { ReferenceElement } from "@floating-ui/dom";
import type { EditorView } from "prosemirror-view";
import {
	type Accessor,
	createContext,
	createSignal,
	onCleanup,
	type ParentComponent,
	type Setter,
	useContext,
} from "solid-js";

export type FormattingToolbarState = {
	visible: boolean;
	top: number;
	left: number;
	reference: ReferenceElement | null;
	view: EditorView | null;
};

export type FormattingToolbarContextT = {
	toolbar: Accessor<FormattingToolbarState>;
	setToolbar: Setter<FormattingToolbarState>;
	showToolbar: (reference: ReferenceElement, view: EditorView) => void;
	hideToolbar: () => void;
};

const FormattingToolbarContext = createContext<FormattingToolbarContextT>();

export const FormattingToolbarProvider: ParentComponent = (props) => {
	const [toolbar, setToolbar] = createSignal<FormattingToolbarState>({
		visible: false,
		top: 0,
		left: 0,
		reference: null,
		view: null,
	});

	let showTimeout: ReturnType<typeof setTimeout> | undefined;

	const clearShowTimeout = () => {
		if (showTimeout !== undefined) {
			clearTimeout(showTimeout);
			showTimeout = undefined;
		}
	};

	onCleanup(clearShowTimeout);

	const showToolbar = (reference: ReferenceElement, view: EditorView) => {
		clearShowTimeout();
		showTimeout = setTimeout(() => {
			showTimeout = undefined;
			setToolbar({ visible: true, top: 0, left: 0, reference, view });
		}, 100);
	};

	const hideToolbar = () => {
		clearShowTimeout();
		setToolbar((prev) => ({ ...prev, visible: false }));
	};

	return (
		<FormattingToolbarContext.Provider
			value={{ toolbar, setToolbar, showToolbar, hideToolbar }}
		>
			{props.children}
		</FormattingToolbarContext.Provider>
	);
};

export const useFormattingToolbar = () => {
	const context = useContext(FormattingToolbarContext);
	if (!context) {
		throw new Error(
			"useFormattingToolbar must be used within a FormattingToolbarProvider",
		);
	}
	return context;
};
