import { createContext, useContext } from "solid-js";

/**
 * Whether the containing message's content should be keyboard-tabbable.
 */
const MessageTabbableContext = createContext<() => boolean>();

export const useMessageTabbable = () => {
	const ctx = useContext(MessageTabbableContext);
	return ctx ?? (() => true);
};

export { MessageTabbableContext };
