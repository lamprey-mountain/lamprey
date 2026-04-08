/**
 * Unicode regex for detecting emoji characters.
 * Used by both the editor emoji plugin and markdown turndown conversion.
 */
export const EMOJI_TEST = /\p{Emoji_Presentation}|\p{Extended_Pictographic}/u;
