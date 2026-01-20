import type * as Y from "yjs";
import { createContext, useContext } from "solid-js";
import { SetStoreFunction, Store } from "solid-js/store";

export type DocumentState = {
	/** currently focused branch */
	branchId: string;

	/** per-branch data */
	branches: Record<string, DocumentBranchState>;
};

type DocumentBranchState = {
	doc: Y.Doc;
	scroll_pos?: number;
};

export function createInitialDocumentState(channelId: string): DocumentState {
	return {
		branchId: channelId,
		branches: {},
	};
}

export type DocumentContextT = [
	Store<DocumentState>,
	SetStoreFunction<DocumentState>,
];

export const DocumentContext = createContext<DocumentContextT>();
export const useDocument = () => useContext(DocumentContext)!;
