import { createContext, ParentProps, useContext } from "solid-js";
import { createStore } from "solid-js/store";
import type { Modal } from "../context";

export type ModalsController = {
	close: () => void;
	open: (modal: Modal) => void;
	alert: (text: string) => void;
	prompt: (text: string, cont: (text: string | null) => void) => void;
	confirm: (text: string, cont: (confirmed: boolean) => void) => void;
};

type ModalsContextType = [Modal[], ModalsController];

const ModalsContext = createContext<ModalsContextType>();

export const ModalsProvider = (p: ParentProps) => {
	const [modals, setModals] = createStore<Modal[]>([]);

	const controller: ModalsController = {
		close() {
			setModals((prev) => prev.slice(1));
		},
		open(modal: Modal) {
			setModals((prev) => [...prev, modal]);
		},
		alert(text: string) {
			setModals((prev) => [{ type: "alert", text } as Modal, ...prev]);
		},
		prompt(text: string, cont: (text: string | null) => void) {
			const modal = {
				type: "prompt" as const,
				text,
				cont,
			};
			setModals((prev) => [modal as Modal, ...prev]);
		},
		confirm(text: string, cont: (confirmed: boolean) => void) {
			const modal = {
				type: "confirm" as const,
				text,
				cont,
			};
			setModals((prev) => [modal as Modal, ...prev]);
		},
	};

	// TEMP: for debugging
	globalThis.modalctl = controller;

	return (
		<ModalsContext.Provider value={[modals, controller]}>
			{p.children}
		</ModalsContext.Provider>
	);
};

export const useModals = (): ModalsContextType => {
	const context = useContext(ModalsContext);
	if (!context) {
		throw new Error("useModals must be used within a ModalsProvider");
	}
	return context;
};
