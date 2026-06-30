import { createContext, createSignal, type JSX, useContext } from "solid-js";
import type { Message as MessageT } from "sdk";

export type MessageToolbarTarget = {
	message: MessageT;
	element: HTMLElement;
};

export type MessageToolbarContextValue = {
	target: () => MessageToolbarTarget | null;
	setTarget: (target: MessageToolbarTarget | null) => void;
};

const MessageToolbarContext = createContext<MessageToolbarContextValue>();

export const MessageToolbarProvider = (props: { children: JSX.Element }) => {
	const [target, setTarget] = createSignal<MessageToolbarTarget | null>(null);

	const value = {
		target,
		setTarget,
	};

	return (
		<MessageToolbarContext.Provider value={value}>
			{props.children}
		</MessageToolbarContext.Provider>
	);
};

export const useMessageToolbar = () => {
	const context = useContext(MessageToolbarContext);
	if (!context) {
		throw new Error(
			"useMessageToolbar must be used within a MessageToolbarProvider",
		);
	}
	return context;
};
