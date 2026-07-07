// TODO: move to emoji package or lib/emoji.ts?

/**
 * Unicode regex for detecting emoji characters.
 * Used by both the editor emoji plugin and markdown turndown conversion.
 */
export const EMOJI_TEST = /\p{Emoji_Presentation}|\p{Extended_Pictographic}/u;

export const CUSTOM_EMOJI_REGEX = /<(a?):(\w+):([0-9a-fA-F-]{32,38})>/g;

/**
 * Count emoji in a message content string.
 * Returns the count of emoji (both custom and unicode) if the message
 * contains ONLY emoji and whitespace, otherwise returns 0.
 */
export function countEmojiOnly(content: string): number {
	const trimmed = content.trim();
	if (!trimmed) return 0;

	// Count and remove custom emoji <:name:id> or <a:name:id>
	const customEmojiMatches = trimmed.match(CUSTOM_EMOJI_REGEX) || [];
	let emojiCount = customEmojiMatches.length;
	const withoutCustomEmoji = trimmed.replace(CUSTOM_EMOJI_REGEX, "");

	// Remove whitespace for checking
	const withoutWhitespace = withoutCustomEmoji.replace(/\s+/g, "");

	// If nothing left after removing custom emoji and whitespace, check custom emoji count
	if (withoutWhitespace.length === 0) {
		return emojiCount;
	}

	// Count unicode emoji using grapheme segmentation
	// This properly handles emoji with variation selectors, ZWJ sequences, etc.
	const segmenter = new Intl.Segmenter("en", {
		granularity: "grapheme",
	});
	for (const { segment } of segmenter.segment(withoutCustomEmoji)) {
		// Skip whitespace
		if (/\s/.test(segment)) continue;
		// Check if this grapheme contains emoji
		if (EMOJI_TEST.test(segment)) {
			emojiCount++;
		} else {
			// Non-emoji content found
			return 0;
		}
	}

	return emojiCount;
}
