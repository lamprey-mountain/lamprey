export type LabelPart =
	| string
	| {
			// type: LabelType;
			type: string;
			value: string;
			user?: User;
			channel?: ThreadT;
			negated?: boolean;
			parts?: LabelPart[];
	  };
