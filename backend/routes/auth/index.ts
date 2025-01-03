import { OpenAPIHono } from "@hono/zod-openapi";
import { data, discord, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { AuthDiscordFinish, AuthDiscordStart } from "./def.ts";
import { UserFromDb } from "../../types/db.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	// app.openapi(AuthLogin, async (c) => {
	//   const req = await c.req.json();
	//   const userRow = db.prepareQuery("SELECT email FROM users WHERE email = ?").firstEntry([req.email]);
	//   if (!userRow) return c.json({ error: "Incorrect password" }, 401);
	//   const user_id = userRow.user_id as string;
	//   const pwRow = db.prepareQuery("SELECT data FROM auth WHERE user_id = ? AND type = ?").firstEntry([user_id, "password"]);
	//   if (!pwRow) return c.json({ error: "Incorrect password" }, 401);
	//   if (!await bcrypt.compare(req.password, pwRow.data as string)) return c.json({ error: "Incorrect password" }, 403);
	//   const sessionRow = db.prepareQuery(`
	//     INSERT INTO sessions (session_id, user_id, token, status)
	//     VALUES (?, ?, ?, ?)
	//     RETURNING *
	//   `).firstEntry([uuidv7(), user_id, crypto.randomUUID(), SessionStatus.Default])!;
	//   return c.json(sessionRow, 201);
	// });

	// TODO: proper auth
	//   app.openapi(withAuth(AuthPasswordDo, { strict: false }), async (c) => {
	//     const req = await c.req.json();
	//     const user_id = c.get("user_id");
	//     const row = db.prepareQuery("SELECT data FROM auth WHERE user_id = ? AND type = ?").first([user_id, "password"]);
	//     if (!row) return c.json({ error: "Incorrect password" });
	//     await bcrypt.compare(req)
	//     throw "todo"
	//   });

	//   app.openapi(AuthPasswordSet, async (c) => {
	//     const req = await c.req.json();
	// // bcrypt.hash
	//     throw "todo"
	//   });

	//   app.openapi(AuthTotpDo, async (c) => {
	//     const req = await c.req.json();
	//     throw "todo"
	//   });

	//   app.openapi(AuthTotpSet, async (c) => {
	//     const req = await c.req.json();
	//     throw "todo"
	//   });

	const validStates = new Set();
	app.openapi(AuthDiscordStart, (c) => {
		const state = uuidv7();
		validStates.add(state);
		return c.redirect(discord.buildUrl(state), 302);
	});

	app.openapi(AuthDiscordFinish, async (c) => {
		const state = c.req.query("state");
		const code = c.req.query("code");
		if (!validStates.has(state)) return c.text("invalid state", 400);
		if (!code) return c.text("missing code", 400);
		validStates.delete(state);
		const { access_token } = await discord.exchangeCodeForToken(code);
		const { user: discordUser } = await discord.getUser(access_token);
		console.log(discordUser);
		let user = await data.userSelectByDiscordId(discordUser.id);
		console.log(user);
		if (!user) {
			user = await data.userInsert(uuidv7(), {
				name: discordUser.global_name ?? discordUser.username,
				description: null,
				status: null,
				is_bot: false,
				is_alias: false,
			}, {
				parent_id: null,
				is_system: false,
				can_fork: false,
				discord_id: discordUser.id,
			});
			console.log(user);
		}
		const token = crypto.randomUUID();
		const session = await data.sessionInsert({
			id: uuidv7(),
			user_id: user.id,
			token,
			status: 1,
		});
		console.log(session);
		return c.html(`
      <script>
        localStorage.setItem("token", "${session.token}");
        localStorage.setItem("user_id", "${user.id}");
        location.href = "/";
      </script>`);
	});
}
