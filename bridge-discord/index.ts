import { DiscordIntent, MessageT } from "./types.ts";
import { DB } from "sqlite";

const db = new DB("data.db");
const BASE_URL = Deno.env.get("BASE_URL")!;
const MY_TOKEN = Deno.env.get("MY_TOKEN")!;
const DISCORD_TOKEN = Deno.env.get("DISCORD_TOKEN")!;

let myws: WebSocket;
let dcws: WebSocket;

db.execute(`
  CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT);
  CREATE TABLE IF NOT EXISTS messages (chat_id TEXT, discord_id TEXT, PRIMARY KEY (chat_id, discord_id));
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
  	myws.send(JSON.stringify({ type: "hello", token: MY_TOKEN }));
  });

  myws.addEventListener("message", (e) => {
  	const msg = JSON.parse(e.data);
	
  	if (msg.type === "ping") {
  		myws.send(JSON.stringify({ type: "pong" }));
  	} else if (msg.type === "ready") {
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

reconnectChat();
reconnectDiscord();

const dtoc = new Map([
  ["777553502431084565",  "019438f6-bcb4-7d30-ba05-f55cfa4c61d2"], // inspirational-quotes
  ["854134072322424832",  "0194391b-9765-7e45-bd7d-872b005c4d00"], // spam
  ["849816400251584582",  "01943d75-ac89-7869-8112-bd6a5a09cab9"], // brake-cusine
  ["862392374331310100",  "01943d75-c674-7c94-b90f-169b92f5e05a"], // motivational-quotes
  ["1318306193248092271", "01943d75-e79b-74d6-93e1-1a1c48d49bfe"], // side projects
  ["1320777240778113045", "01943d76-ad79-718f-9387-946138f8dfd1"], // discord if it was good
  ["977802452076216360",  "019439e6-5c36-7914-ba30-008e46a1d67f"], // testing
  ["663854113418641429",  "01943dcb-bb66-7210-9cda-77a1001881eb"], // genprog
]);

const ctod = new Map(dtoc.entries().map(([k, v]) => [v, k]));

const discordWebhooks = new Map([
  ["777553502431084565",  "https://canary.discord.com/api/webhooks/1325880931424407604/ms-qfq8J8l-RhUh2SyqaoaEjWEaF_IRoEm5_1MQ7EChs0j9UeD97IZmK6iIYi-onxAuU?wait=true"], // inspirational-quotes
  ["854134072322424832",  "https://canary.discord.com/api/webhooks/1325935866945863750/_2MCaaM9fWe6zuq_fovuzro4ylKTpknY94gLfhUnCM9CabpnL-jKqDIIlLiHU0MS_2gl?wait=true"], // spam
  ["849816400251584582",  "https://canary.discord.com/api/webhooks/1325935968544358491/hsDSoHrwdVQEua6nYpXRYcxAEbofMiGDUGLy4HSU6wLqkyXKKdmn141SieS0iKjmZLIj?wait=true"], // brake-cusine
  ["862392374331310100",  "https://canary.discord.com/api/webhooks/1325936011695358066/7pLkACUQtyq_hbI9xiI8f_4mGofXxqMEicyc1a49IduodXV1ufQ6ehceXYZ6m246tTxL?wait=true"], // motivational-quotes
  ["1318306193248092271", "https://canary.discord.com/api/webhooks/1318871890671964200/8bdO2LoqmN2Sio1hXVbWB952D0MBH0k4aDmZw775M9izrNiwkVpjaN11XjXdj4Be48sQ?wait=true&thread_id=1318306193248092271"], // side projects
  ["1320777240778113045", "https://canary.discord.com/api/webhooks/1318871890671964200/8bdO2LoqmN2Sio1hXVbWB952D0MBH0k4aDmZw775M9izrNiwkVpjaN11XjXdj4Be48sQ?wait=true&thread_id=1320777240778113045"], // discord if it was good
  ["977802452076216360",  "https://canary.discord.com/api/webhooks/1325959282235146332/s60tkbc7JY_oN3yRYi5pS-jNlqacZWu4XgxasHbU1751KHuS7egpGuPWA7keA_F5BjSS?wait=true"], // testing
  ["663854113418641429",  "https://canary.discord.com/api/webhooks/1273279371469389918/pDd3SnaYZWN1Xhh4uc0IICWyUklpcIBHwXqx81JJm96L_XMc4Lg3wk5IGjGdu8MyY6Rk?wait=true"], // genprog
]);

const guild_ids = new Map([
  ["777553502431084565",  "777553454063026226"], // inspirational-quotes
  ["854134072322424832",  "777553454063026226"], // spam
  ["849816400251584582",  "777553454063026226"], // brake-cusine
  ["862392374331310100",  "777553454063026226"], // motivational-quotes
  ["1318306193248092271", "777553454063026226"], // side projects
  ["1320777240778113045", "777553454063026226"], // discord if it was good
  ["977802452076216360",  "977802451585470474"], // testing
  ["663854113418641429",  "391020510269669376"], // genprog
]);

const dcHookIds = new Set(discordWebhooks.values().map(i => i.match(/webhooks\/([0-9]+)\//)?.[1]!));

async function handleChat(msg: any) {
  console.log("chat:", msg.type);
	if (msg.type === "upsert.message") {
	  const message: MessageT = msg.message;
	  if (message.author.id === "01943cc1-62e0-7c0e-bb9b-a4ff42864d69") return;
	  const channel_id = ctod.get(message.thread_id);
	  if (!channel_id) return;
    const reply_ids = db.prepareQuery("SELECT * FROM messages WHERE chat_id = ?").firstEntry([message.reply_id]);
    let embeds;
    if (reply_ids) {
      const { discord_id, chat_id } = reply_ids as any;
  	  const req = await fetch(`https://chat.celery.eu.org/api/v1/threads/${message.thread_id}/messages/${chat_id}`, {
  	    headers: {
  	      "Authorization": MY_TOKEN,
	      }
      });
      const reply: MessageT = await req.json();
      embeds = [{
        description: `**[replying to ${reply.override_name || reply.author.name}](https://canary.discord.com/channels/${guild_ids.get(channel_id)}/${channel_id}/${discord_id})**\n${reply.content}`,
      }];
    }
	  const req = await fetch(discordWebhooks.get(channel_id)!, {
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
    db.prepareQuery("INSERT INTO messages (chat_id, discord_id) VALUES (?, ?)").execute([message.id, d.id]);
	}
}

async function handleDiscord(msg: any) {
  console.log("discord:", msg.t);
  if (msg.t === "TYPING_START") {
    // console.log(msg)
  } else if (msg.t === "MESSAGE_CREATE") {
    if (dcHookIds.has(msg.d.webhook_id)) return;
    const thread_id = dtoc.get(msg.d.channel_id);
    if (!thread_id) return;
    const reply_id_discord = msg.d.message_reference?.type === 0 ? msg.d.message_reference.message_id : null;
    const reply_id = db.prepareQuery("SELECT * FROM messages WHERE discord_id = ?").firstEntry([reply_id_discord])?.chat_id ?? null;
	  const req = await fetch(`https://chat.celery.eu.org/api/v1/threads/${thread_id}/messages`, {
	    method: "POST",
	    headers: {
	      "Content-Type": "application/json",
	      "Authorization": MY_TOKEN,
	    },
	    body: JSON.stringify({
    	  content: msg.d.content || "(no content?)",
    	  override_name: msg.d.member?.nick ?? msg.d.author.global_name ?? msg.d.author.username,
    	  reply_id,
	    }),
	  });
	  const d = await req.json();
    db.prepareQuery("INSERT INTO messages (chat_id, discord_id) VALUES (?, ?)").execute([d.id, msg.d.id]);
  }
}
