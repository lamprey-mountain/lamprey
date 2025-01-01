import { OpenAPIHono } from "@hono/zod-openapi";
import { db, discord, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { AuthDiscordFinish, AuthDiscordStart } from "./def.ts";

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
    const { user } = await discord.getUser(access_token);
    console.log(user);
    let row = db.prepareQuery("SELECT * FROM users WHERE discord_id = ?")
      .firstEntry([user.id]);
    if (!row) {
      row = db.prepareQuery(`
        INSERT INTO users (user_id, parent_id, name, description, status, is_bot, is_alias, is_system, can_fork, discord_id)
        VALUES (:user_id, :parent_id, :name, :description, :status, :is_bot, :is_alias, :is_system, :can_fork, :discord_id)
        RETURNING *
      `).firstEntry({
        user_id: uuidv7(),
        parent_id: null,
        name: user.global_name ?? user.username,
        description: null,
        status: null,
        is_bot: false,
        is_alias: false,
        is_system: false,
        can_fork: false,
        discord_id: user.id,
      })!;
    }
    const token = crypto.randomUUID();
    db.prepareQuery(`
      INSERT INTO sessions (session_id, user_id, token, status)
      VALUES (?, ?, ?, ?)
    `).execute([uuidv7(), row.user_id, token, 1]);
    return c.html(`
      <script>
        localStorage.setItem("token", "${token}");
        localStorage.setItem("user_id", "${row.user_id}");
        location.href = "/";
      </script>`
    );
  });
}
