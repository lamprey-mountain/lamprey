export type { HeaderItem } from "@lamprey/markdown";

export type ContextId = {
	channelId: string;
	branchId: string;
};

export type ChangesetSelection = {
	start_seq: number;
	end_seq: number;
};

export type DocumentMode = "edit" | "diff_preview" | "diff_readonly";
