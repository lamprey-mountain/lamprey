import { DiscordIntent, MessageT } from "./types.ts";
import { DB } from "sqlite";
import { portals } from "./config.ts";

const db = new DB("data.db");
const BASE_URL = Deno.env.get("BASE_URL")!;
const MY_TOKEN = Deno.env.get("MY_TOKEN")!;
const DISCORD_TOKEN = Deno.env.get("DISCORD_TOKEN")!;

let myws: WebSocket;
let dcws: WebSocket;

db.execute(`
  CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT);
  CREATE TABLE IF NOT EXISTS messages (chat_id TEXT, discord_id TEXT, chat_thread_id TEXT, discord_channel_id TEXT, PRIMARY KEY (chat_id, discord_id));
  CREATE TABLE IF NOT EXISTS attachments (chat_id TEXT, discord_id TEXT, PRIMARY KEY (chat_id, discord_id));
`);

const _set = db.prepareQuery<[string, string | null]>("INSERT OR REPLACE INTO config VALUES (?, ?)")
const _get = db.prepareQuery<[string], { value: string | null }>("SELECT value FROM config WHERE key = ?")
const set = (key: string, value: string | null) => _set.execute([key, value]);
const get = (key: string) => _get.firstEntry([key])?.value ?? null;

function reconnectDiscord() {
  const url = new URL(get("discordGatewayUrl") ?? `wss://gateway.discord.gg`);
  url.searchParams.set("v", "10");
  url.searchParams.set("encoding", "json");
  dcws = new WebSocket(url);
  dcws.addEventListener("message", (e) => {
    const msg = JSON.parse(e.data);
    if (msg.s) set("discordLastSeq", msg.s.toString());
    if (msg.op === 10) {
      setInterval(() => {
        dcws.send(JSON.stringify({
          op: 1,
          d: get("discordLastSeq"),
        }));
      }, msg.d.heartbeat_interval);
      const sid = get("discordSessionId")
      if (sid) {
        dcws.send(JSON.stringify({
          op: 6,
          d: {
            token: DISCORD_TOKEN,
            session_id: sid,
            seq: parseInt(get("discordLastSeq")!, 10),
          }
        }));
      } else {
        dcws.send(JSON.stringify({
          op: 2,
          d: {
            token: DISCORD_TOKEN,
            intents: DiscordIntent.GUILDS | DiscordIntent.GUILDS_MESSAGES | DiscordIntent.GUILDS_MESSAGE_REACTIONS | DiscordIntent.GUILDS_MESSAGE_TYPING | DiscordIntent.MESSAGE_CONTENT,
            properties: {},
          }
        }));
      }
    } else if (msg.op === 7) {
      dcws.close();
    } else if (msg.op === 9) {
      if (!msg.d) {
        set("discordLastSeq", null);
        set("discordSessionId", null);
      }
      dcws.close();
    } else if (msg.op === 0) {
      if (msg.t === "READY") {
        console.log("discord auth", msg.d.user.username);
        set("discordGatewayUrl", msg.d.resume_gateway_url);
        set("discordSessionId", msg.d.session_id);
      }
      handleDiscord(msg);
    }
  });
  
  dcws.addEventListener("close", () => {
    setTimeout(reconnectDiscord, 1000);
  });
}

function reconnectChat() {
  myws = new WebSocket(`${BASE_URL}/api/v1/sync`);
  myws.addEventListener("open", () => {
  	console.log("chat opened");
  	myws.send(JSON.stringify({ type: "Hello", token: MY_TOKEN }));
  });

  myws.addEventListener("message", (e) => {
  	const msg = JSON.parse(e.data);
	
  	if (msg.type === "Ping") {
  		myws.send(JSON.stringify({ type: "Pong" }));
  	} else if (msg.type === "Ready") {
  		console.log("chat auth", msg.user.name);
		} else {
		  handleChat(msg)
		}
  });

  myws.addEventListener("close", () => {
    console.log("chat closed");
    setTimeout(reconnectChat, 1000);
  });
}

const dtoc = new Map(portals.map(i => [i.discord_channel_id, i.my_thread_id]));
const ctod = new Map(portals.map(i => [i.my_thread_id, i.discord_channel_id]));
const discordWebhooks = new Map(portals.map(i => [i.discord_channel_id, i.discord_webhook]));
const guild_ids = new Map(portals.map(i => [i.discord_channel_id, i.discord_guild_id]));
const locks = new Map<string, Promise<unknown>>();

async function backfill(dcChannelId: string, from: string, to: string) {
  const batch = await fetch(`https://canary.discord.com/api/v9/channels/${dcChannelId}/messages?after=${from}`, {
    headers: {
      "Authorization": "Bot " + DISCORD_TOKEN,
    }
  }).then(res => res.json());
  console.log(batch);
  for (const msg of batch) {
    await handleDiscordMessage(msg);
  }
  if (batch.length && batch.at(-1).id !== to) {
    await backfill(dcChannelId, batch.at(-1).id, to);
  }
}

reconnectChat();
reconnectDiscord();

async function handleChat(msg: any) {
  console.log("chat:", msg.type);
	if (msg.type === "Webhook") {
    console.log("webhook:", msg);
	} else if (msg.type === "UpsertMessage") {
	  const message: MessageT = msg.message;
	  if (message.author.id === "01943cc1-62e0-7c0e-bb9b-a4ff42864d69") return;
	  const channel_id = ctod.get(message.thread_id);
	  if (!channel_id) return;
    const reply_ids = db.prepareQuery("SELECT * FROM messages WHERE chat_id = ?").firstEntry([message.reply_id]);
    let embeds;
    if (reply_ids) {
      const { discord_id, chat_id } = reply_ids as any;
  	  const req = await fetch(`https://chat.celery.eu.org/api/v1/thread/${message.thread_id}/message/${chat_id}`, {
  	    headers: {
  	      "Authorization": MY_TOKEN,
          "content-type": "application/json",
	      }
      });
      const reply: MessageT = await req.json();
      embeds = [{
        description: `**[replying to ${reply.override_name || reply.author.name}](https://canary.discord.com/channels/${guild_ids.get(channel_id)}/${channel_id}/${discord_id})**\n${reply.content}`,
      }];
    }
    const p = Promise.withResolvers();
    locks.set(channel_id, p.promise);
    const chat_id = discordWebhooks.get(channel_id)!;
	  const req = await fetch(chat_id, {
	    method: "POST",
	    headers: {
	      "Content-Type": "application/json",
	    },
	    body: JSON.stringify({
    	  content: message.content || "(no content?)",
    	  username: message.override_name || message.author.name,
    	  allowed_mentions: { replied_user: true },
    	  embeds,
	    }),
	  });
	  console.log("webhook", req.status, {
  	  content: message.content || "(no content?)",
  	  username: message.override_name || message.author.name,
  	  allowed_mentions: { replied_user: true },
  	  embeds,
	  });
	  const d = await req.json();
    db.prepareQuery("INSERT INTO messages (chat_id, discord_id, chat_thread_id, discord_channel_id) VALUES (?, ?, ?, ?)").execute([message.id, d.id, chat_id, d.channel_id]);
    for (let i = 0; i < message.attachments.length; i++) {
      db.prepareQuery("INSERT INTO attachments (chat_id, discord_id) VALUES (?, ?)").execute([message.attachments[i].id, d.attachments[i].id]);
    }
    p.resolve(null);
	}
}

async function handleDiscordMessage(msg: any) {
  const thread_id = dtoc.get(msg.channel_id);
  if (!thread_id) return;
  if (db.prepareQuery("SELECT * FROM messages WHERE discord_id = ?").firstEntry([msg.id])) return;
  const attachments = [];
  for (const a of msg.attachments) {
    const blob = await fetch(a.url).then(r => r.blob());
    const upload = await fetch("https://chat.celery.eu.org/api/v1/media", {
      method: "POST",
      headers: {
        "Authorization": MY_TOKEN,
        "content-type": "application/json",
      },
      body: JSON.stringify({
        filename: a.filename,
        size: blob.size,
      }),
    });
    const { media_id: id, upload_url } = await upload.json();
    await fetch(upload_url, {
      method: "PATCH",
      headers: {
        "Authorization": MY_TOKEN,
      },
      body: blob,
    });
    db.prepareQuery("INSERT INTO attachments (chat_id, discord_id) VALUES (?, ?)").execute([id, a.id]);
    attachments.push({ id });
  }
  const reply_id_discord = msg.message_reference?.type === 0 ? msg.message_reference.message_id : null;
  const reply_id = db.prepareQuery("SELECT * FROM messages WHERE discord_id = ?").firstEntry([reply_id_discord])?.chat_id ?? null;
  const req = await fetch(`https://chat.celery.eu.org/api/v1/thread/${thread_id}/message`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Authorization": MY_TOKEN,
    },
    body: JSON.stringify({
  	  content: msg.content || (attachments.length ? null : "(no content?)"),
  	  override_name: msg.member?.nick ?? msg.author.global_name ?? msg.author.username,
  	  reply_id,
  	  attachments,
    }),
  });
  const d = await req.json();
  db.prepareQuery("INSERT INTO messages (chat_id, discord_id, chat_thread_id, discord_channel_id) VALUES (?, ?, ?, ?)").execute([d.id, msg.id, thread_id, msg.channel_id]);
}

const LAST_DC_ID: string = db.prepareQuery<[string]>("SELECT max(discord_id) FROM messages").first([])?.[0] ?? "";

async function handleDiscord(msg: any) {
  console.log("discord:", msg.t, msg.d);
  if (msg.t === "GUILD_CREATE") {
    for (const channel of msg.d.channels) {
      if (dtoc.has(channel.id)) {
        await locks.get(channel.id);
        locks.set(channel.id, backfill(channel.id, LAST_DC_ID, channel.last_message_id));
      }
    }
  } else if (msg.t === "TYPING_START") {
    // console.log(msg)
  } else if (msg.t === "MESSAGE_CREATE") {
    await locks.get(msg.d.channel_id);
    handleDiscordMessage(msg.d);
  } else if (msg.t === "MESSAGE_UPDATE") {
    await locks.get(msg.d.channel_id);
    const message_id = db.prepareQuery("SELECT * FROM messages WHERE discord_id = ?").firstEntry([msg.d.id])?.chat_id ?? null;
    if (!message_id) return;
    const thread_id = dtoc.get(msg.d.channel_id);
    if (!thread_id) return;
    const attachments = [];
    for (const a of msg.d.attachments) {
      const existing = db.prepareQuery("SELECT * FROM attachments WHERE discord_id = ?").firstEntry([a.id]);
      if (existing) {
        attachments.push({ id: existing.chat_id });
        continue;
      } else {
        const blob = await fetch(a.url).then(r => r.blob());
        const upload = await fetch("https://chat.celery.eu.org/api/v1/media", {
          method: "POST",
          headers: {
            "Authorization": MY_TOKEN,
            "content-type": "application/json",
          },
          body: JSON.stringify({
            filename: a.filename,
            size: blob.size,
          }),
        });
        const { media_id: id, upload_url } = await upload.json();
        await fetch(upload_url, {
          method: "PATCH",
          headers: {
            "Authorization": MY_TOKEN,
          },
          body: blob,
        });
        attachments.push({ id });
      }
    }
    const reply_id_discord = msg.d.message_reference?.type === 0 ? msg.d.message_reference.message_id : null;
    const reply_id = db.prepareQuery("SELECT * FROM messages WHERE discord_id = ?").firstEntry([reply_id_discord])?.chat_id ?? null;
    const req = await fetch(`https://chat.celery.eu.org/api/v1/thread/${thread_id}/message/${message_id}`, {
      method: "PATCH",
      headers: {
        "Content-Type": "application/json",
        "Authorization": MY_TOKEN,
      },
      body: JSON.stringify({
    	  content: msg.d.content || (attachments.length ? null : "(no content?)"),
    	  override_name: msg.d.member?.nick ?? msg.d.author.global_name ?? msg.d.author.username,
    	  reply_id,
    	  attachments,
      }),
    });
    const d = await req.json();
    console.log(msg.d);
    console.log(d);
    console.log(attachments)
  } else if (msg.t === "MESSAGE_DELETE") {
    await locks.get(msg.d.channel_id);
    const thread_id = dtoc.get(msg.d.channel_id);
    if (!thread_id) return;
    const message_id = db.prepareQuery("SELECT * FROM messages WHERE discord_id = ?").firstEntry([msg.d.id])?.chat_id ?? null;
    if (!message_id) return;
    await fetch(`https://chat.celery.eu.org/api/v1/thread/${thread_id}/message/${message_id}`, {
      method: "DELETE",
      headers: {
        "Content-Type": "application/json",
        "Authorization": MY_TOKEN,
      },
    });
  }
}
