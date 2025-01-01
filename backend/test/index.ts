const BASE_URL = "http://localhost:8000";

async function http(method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE", url: string, body?: any) {
  console.log(`${method} ${url}`);
  const req = await fetch(`${BASE_URL}${url}`, {
    method,
    headers: {
      "authorization": "abcdefg",
      // "content-type": body ? "application/json" : null,
      "content-type": "application/json",
    },
    body: body ? JSON.stringify(body) : undefined,
  })
  if (!req.ok) throw new Error(`request failed (${req.status}): ${await req.text()}`);
  const json = await req.json();
  // console.log(json);
  return json;
}

async function test() {
  const room = await http("POST", "/api/v1/rooms", { name: "arst" });
  await http("GET", "/api/v1/rooms");
  await http("PATCH", `/api/v1/rooms/${room.room_id}`, { description: "foobar" });
  await http("GET", `/api/v1/rooms/${room.room_id}`);
  const thread = await http("POST", `/api/v1/rooms/${room.room_id}/threads`, { name: "test thread" });
  const message = await http("POST", `/api/v1/rooms/${room.room_id}/threads/${thread.thread_id}/messages`, { content: "hello world" });
  await http("GET", `/api/v1/rooms/${room.room_id}/threads/${thread.thread_id}/messages`);
  await http("PATCH", `/api/v1/rooms/${room.room_id}/threads/${thread.thread_id}/messages/${message.message_id}`, { content: "goodbye world" });
  await http("GET", `/api/v1/rooms/${room.room_id}/threads/${thread.thread_id}/messages`);
}

const ws = new WebSocket(`http://localhost:8000/api/v1/sync`);
ws.onopen = () => {
  console.log("open");
  ws.send(JSON.stringify({ hello: "world" }));
  test();
}
ws.onmessage = (ev) => console.log("ws message", JSON.parse(ev.data));
// ws.close();
