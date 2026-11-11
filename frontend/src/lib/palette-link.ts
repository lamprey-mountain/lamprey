export type PaletteLink = {
	kind: "channel" | "room" | "user" | "invite" | "page";
	name?: string;
	/** path to navigate to */
	path: string;
	channelId?: string;
	roomId?: string;
	messageId?: string;
};

type Pattern = {
	re: RegExp;
	build: (m: RegExpMatchArray) => PaletteLink;
};

const patterns: Pattern[] = [
	{
		re: /channel\/([^/?#]+)\/message\/([^/?#]+)/,
		build: (m) => ({
			kind: "channel",
			channelId: m[1],
			messageId: m[2],
			path: `/channel/${m[1]}/message/${m[2]}`,
		}),
	},
	{
		re: /thread\/([^/?#]+)\/message\/([^/?#]+)/,
		build: (m) => ({
			kind: "channel",
			channelId: m[1],
			messageId: m[2],
			path: `/thread/${m[1]}/message/${m[2]}`,
		}),
	},
	{
		re: /channel\/([^/?#]+)/,
		build: (m) => ({
			kind: "channel",
			channelId: m[1],
			path: `/channel/${m[1]}`,
		}),
	},
	{
		re: /thread\/([^/?#]+)/,
		build: (m) => ({
			kind: "channel",
			channelId: m[1],
			path: `/thread/${m[1]}`,
		}),
	},
	{
		re: /room\/([^/?#]+)/,
		build: (m) => ({
			kind: "room",
			roomId: m[1],
			path: `/room/${m[1]}`,
		}),
	},
	{
		re: /user\/([^/?#]+)/,
		build: (m) => ({ kind: "user", path: `/user/${m[1]}` }),
	},
	{
		re: /invite\/([^/?#]+)/,
		build: (m) => ({ kind: "invite", path: `/invite/${m[1]}` }),
	},
	{
		re: /(?:^|\/)search(?:[/?#]|$)/,
		build: () => ({ kind: "page", path: "/search", name: "search" }),
	},
	{
		re: /(?:^|\/)(inbox|friends|settings)(?:[/?#]|$)/,
		build: (m) => ({ kind: "page", path: `/${m[1]}`, name: m[1] }),
	},
];

const looksLikeUrl = (s: string) =>
	/^(https?:\/\/|www\.)/i.test(s) ||
	/^[a-z0-9-]+(\.[a-z0-9-]+)+(\/|$)/i.test(s);

/**
 * best effort guess of where a pasted link points to.
 * the hostname is ignored, so links copied from other instances or
 * behind a path prefix still resolve.
 */
export function parsePaletteLink(text: string): PaletteLink | null {
	const url = text.trim().split(/\s+/)[0];
	if (!url || !looksLikeUrl(url)) return null;

	let pathname: string;
	try {
		pathname = new URL(/^[a-z]+:\/\//i.test(url) ? url : `https://${url}`)
			.pathname;
	} catch {
		return null;
	}

	for (const { re, build } of patterns) {
		const match = pathname.match(re);
		if (match) return build(match);
	}

	return null;
}
