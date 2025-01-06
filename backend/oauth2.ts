const CLIENT_ID = Deno.env.get("DISCORD_CLIENT_ID")!;
const CLIENT_SECRET = Deno.env.get("DISCORD_CLIENT_SECRET")!;
const REDIRECT_URI = Deno.env.get("DISCORD_REDIRECT_URI")!;

export function buildUrl(state: string): string {
	const url = new URL("https://canary.discord.com/oauth2/authorize");
	url.searchParams.set("client_id", CLIENT_ID);
	url.searchParams.set("response_type", "code");
	url.searchParams.set("redirect_uri", REDIRECT_URI);
	url.searchParams.set("scope", "identify");
	url.searchParams.set("state", state);
	return url.href;
}

export async function exchangeCodeForToken(code: string) {
	const params = new URLSearchParams([
		["grant_type", "authorization_code"],
		["code", code],
		["redirect_uri", REDIRECT_URI],
	]);

	const res = await fetch("https://discord.com/api/v10/oauth2/token", {
		method: "POST",
		headers: {
			"content-type": "application/x-www-form-urlencoded",
			"authorization": "Basic " + btoa(`${CLIENT_ID}:${CLIENT_SECRET}`),
		},
		body: params.toString(),
	});
	return await res.json();
}

export async function getUser(token: string) {
	const res = await fetch("https://discord.com/api/v10/oauth2/@me", {
		method: "GET",
		headers: {
			"authorization": `Bearer ${token}`,
		},
	});
	return await res.json();
}

export async function revokeToken(token: string) {
	const params = new URLSearchParams([
		["token_type_hint", "access_token"],
		["token", token],
	]);

	await fetch("https://discord.com/api/v10/oauth2/token/revoke", {
		method: "POST",
		headers: {
			"content-type": "application/x-www-form-urlencoded",
			"authorization": "Basic " + btoa(`${CLIENT_ID}:${CLIENT_SECRET}`),
		},
		body: params.toString(),
	});
}

// const state = uuidv7();
// const url = buildUrl(state);
// // display url to user...
// // ...receive code from request
// const code = "abc123";
// const { access_token: token } = await exchangeCodeForToken(code);
// const { user } = await getUser(token);
// // do something wtih user.id, etc
// await revokeToken(token);
