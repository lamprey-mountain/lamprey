// Type declaration for Intl.Segmenter (ESNext.Intl)
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/Segmenter

interface IntlSegmenterOptions {
	granularity?: "grapheme" | "word" | "sentence";
	localeMatcher?: "lookup" | "best fit";
	usage?: "sort" | "standalone";
}

interface SegmentResult {
	segment: string;
	index: number;
	input: string;
	isWordLike?: boolean;
}

interface IntlSegmenter {
	segment(input: string): IterableIterator<SegmentResult>;
	resolvedOptions(): {
		locale: string;
		granularity: string;
	};
}

interface Intl {
	Segmenter: {
		prototype: IntlSegmenter;
		new (
			locale?: string | string[],
			options?: IntlSegmenterOptions,
		): IntlSegmenter;
		supportedLocalesOf(locales: string | string[]): string[];
	};
}
