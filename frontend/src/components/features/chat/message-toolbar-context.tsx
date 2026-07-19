import type { Message as MessageT } from "sdk";
import { createContext, createSignal, type JSX, useContext } from "solid-js";

export type MessageToolbarTarget = {
	message: MessageT;
	element: HTMLElement;
};

export type MessageToolbarContextValue = {
	target: () => MessageToolbarTarget | null;
	setTarget: (target: MessageToolbarTarget | null) => void;
	containerRef: () => HTMLElement | undefined;
	setContainerRef: (el: HTMLElement | undefined) => void;
	locked: () => boolean;
	setLocked: (locked: boolean) => void;
};

const MessageToolbarContext = createContext<MessageToolbarContextValue>();

export const MessageToolbarProvider = (props: { children: JSX.Element }) => {
	const [target, setTarget] = createSignal<MessageToolbarTarget | null>(null);
	const [containerRef, setContainerRef] = createSignal<HTMLElement>();
	const [locked, setLocked] = createSignal(false);

	const setTargetLocked = (t: MessageToolbarTarget | null) => {
		if (locked()) return;
		setTarget(t);
	};

	// NOTE: maybe use a store here
	const value = {
		target,
		setTarget: setTargetLocked,
		containerRef,
		setContainerRef,
		locked,
		setLocked,
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
