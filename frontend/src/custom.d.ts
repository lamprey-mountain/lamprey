declare module "*.png" {
	const value: string;
	export default value;
}
declare module "*.scss" {
	const value: string;
	export default value;
}
declare module "*.svg" {
	const value: string;
	export default value;
}
declare module "*.html?raw" {
	const content: string;
	export default content;
}

declare namespace Intl {
	interface SegmenterOptions {
		granularity?: "grapheme" | "word" | "sentence";
		localeMatcher?: "lookup" | "best fit";
	}

	interface SegmentResult {
		segment: string;
		index: number;
		input: string;
		isWordLike?: boolean;
	}

	interface Segments {
		[Symbol.iterator](): IterableIterator<SegmentResult>;
		containing(index: number): SegmentResult;
	}

	interface Segmenter {
		segment(input: string): Segments;
		resolvedOptions(): {
			locale: string;
			granularity: "grapheme" | "word" | "sentence";
		};
	}

	const Segmenter: {
		prototype: Segmenter;
		new (locale?: string | string[], options?: SegmenterOptions): Segmenter;
		supportedLocalesOf(locales: string | string[]): string[];
	};
}
