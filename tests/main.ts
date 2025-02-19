import { assertEquals } from "@std/assert";

console.log("checking permissions");

const USER0 = Deno.env.get("TOKEN_U0");
const USER1 = Deno.env.get("TOKEN_U1");
console.log(USER0)
console.log(USER1)

const BASE_URL = `${Deno.env.get("BASE_URL")}/api/v1`;

const mkf = (t: string) => async (url: string, init) => {
  const who = t === USER0 ? "u0" : "u1";
  console.log(`${who}: ${init.method ?? "GET"} ${url}`);
  return fetch(`${BASE_URL}${url}`, {
    headers: {
      "authorization": `Bearer ${t}`,
      "content-type": "application/json",
    },
    ...init,
  }).then(async (r) => ({
    status: r.status,
    data: r.status === 204 ? null : await r.json(),
  }));
};

const f0 = mkf(USER0);
const f1 = mkf(USER1);

const room_id = "0194f5be-a340-7011-b7c9-191a29ff5658";
const role_id = "0194f5be-a34b-75e3-9738-d6bd6dd5764c";
const u1_id = "0194f5ba-89e0-7c62-8776-8873b37a6d0a";

async function testPermissions() {
  const a = await f0(
    `/room/${room_id}/thread`,
    {
      method: "POST",
      body: JSON.stringify({ name: "testing" }),
    },
  );

  assertEquals(a.status, 201);
  const tid = a.data.id;

  const b = await f1(
    `/thread/${tid}`,
    { method: "DELETE" },
  );
  assertEquals(b.status, 403);

  const c = await f0(
    `/room/${room_id}/role/${role_id}/member/${u1_id}`,
    { method: "PUT" },
  );
  assertEquals(c.status, 200);

  const d = await f1(`/thread/${tid}`, { method: "DELETE" });
  assertEquals(d.status, 204);

  const e = await f0(
    `/room/${room_id}/thread`,
    {
      method: "POST",
      body: JSON.stringify({ name: "testing" }),
    },
  );

  assertEquals(e.status, 201);
  const tid2 = e.data.id;

  const f = await f0(
    `/room/${room_id}/role/${role_id}/member/${u1_id}`,
    { method: "DELETE" },
  );
  assertEquals(f.status, 200);

  const g = await f1(
    `/thread/${tid2}`,
    { method: "DELETE" },
  );
  assertEquals(g.status, 403);

  const h = await f0(
    `/thread/${tid2}`,
    { method: "DELETE" },
  );
  assertEquals(h.status, 204);
}

testPermissions();
