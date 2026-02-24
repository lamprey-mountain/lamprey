import {
	type Accessor,
	createContext,
	createSignal,
	type ParentComponent,
	type Setter,
	useContext,
} from "solid-js";
import type { ReferenceElement } from "@floating-ui/dom";

export type FormattingToolbarState = {
	visible: boolean;
	top: number;
	left: number;
	reference: ReferenceElement | null;
};

export type FormattingToolbarContextT = {
	toolbar: Accessor<FormattingToolbarState>;
	setToolbar: Setter<FormattingToolbarState>;
	showToolbar: (reference: ReferenceElement) => void;
	hideToolbar: () => void;
};

const FormattingToolbarContext = createContext<FormattingToolbarContextT>();

export const FormattingToolbarProvider: ParentComponent = (props) => {
	const [toolbar, setToolbar] = createSignal<FormattingToolbarState>({
		visible: false,
		top: 0,
		left: 0,
		reference: null,
	});

	const showToolbar = (reference: ReferenceElement) => {
		setToolbar({ visible: true, top: 0, left: 0, reference });
	};

	const hideToolbar = () => {
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
