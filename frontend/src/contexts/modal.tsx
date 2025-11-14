import { createContext, ParentProps, useContext } from "solid-js";
import { createStore } from "solid-js/store";
import type { Modal } from "../context";

const ModalsContext = createContext();

export const ModalsProvider = (p: ParentProps) => {
	const [modals, setModals] = createStore<Modal[]>([]);

	const controller = {
		close() {
			setModals(prev => prev.slice(1));
		},
		open(modal: Modal) {
			setModals(prev => [...prev, modal]);
		},
		alert(text: string) {
			setModals(prev => [{ type: "alert", text } as Modal, ...prev]);
		},
		prompt(text: string, cont: (text: string | null) => void) {
			const modal = {
				type: "prompt" as const,
				text,
				cont,
			};
			setModals(prev => [modal as Modal, ...prev]);
		},
		confirm(text: string, cont: (confirmed: boolean) => void) {
			const modal = {
				type: "confirm" as const,
				text,
				cont,
			};
			setModals(prev => [modal as Modal, ...prev]);
		},
	};

	return (
		<ModalsContext.Provider value={[modals, controller]}>
			{p.children}
		</ModalsContext.Provider>
	);
};

export const useModals = () => {
	return useContext(ModalsContext)!;
};
