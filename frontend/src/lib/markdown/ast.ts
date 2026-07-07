export type MentionData =
	| { type: "User"; id: string }
	| { type: "Role"; id: string }
	| { type: "Channel"; id: string }
	| { type: "Everyone" };

export type SerializedInline =
	| { type: "Strong"; children: SerializedInline[] }
	| { type: "Emphasis"; children: SerializedInline[] }
	| { type: "Strikethrough"; children: SerializedInline[] }
	| { type: "Link"; href: string; children: SerializedInline[] }
	| { type: "Spoiler"; children: SerializedInline[] }
	| { type: "Code"; children: SerializedInline[] }
	| { type: "Text"; content: string }
	| { type: "Mention"; mention: MentionData }
	| { type: "CustomEmoji"; animated: boolean; name: string; id: string }
	| { type: "UnicodeEmoji"; content: string };

export type SerializedBlock =
	| { type: "Header"; level: number; children: SerializedInline[] }
	| { type: "Paragraph"; children: SerializedInline[] }
	| { type: "Blockquote"; children: SerializedBlock[] }
	| { type: "Codeblock"; language: string | null; content: string }
	| { type: "List"; items: SerializedBlock[] }
	| { type: "ListItem"; content: SerializedBlock[] }
	| {
			type: "Table";
			header: SerializedInline[][];
			rows: SerializedInline[][][];
	  };

export type SerializedDocument = { blocks: SerializedBlock[] };
