export type LabelPart =
	| string
	| LabelPart[]
	| {
			type: string;
			value: string;
			user?: User;
			channel?: ThreadT;
			negated?: boolean;
			parts?: LabelPart[];
	  };
