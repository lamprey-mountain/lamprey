import { OpenAPIHono, z } from "@hono/zod-openapi";
import { auth, withAuth } from "../auth.ts";
import { blobs, data, events, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { Room } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { MediaCreate } from "./def.ts";
import { bodyLimit } from "hono/body-limit";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	// app.openapi(withAuth(MediaCreate), async (c) => {
	//   const r = await c.req.json();
	//   // blobs.
	// 	// const perms = c.get("permissions");
	// 	// if (!perms.has("View")) return c.json({ error: "not found" }, 404);
	// 	// const room_id = c.req.param("room_id")!;
	// 	// const roles = await data.roleList(room_id, {
	// 	// 	limit: parseInt(c.req.query("limit") ?? "10", 10),
	// 	// 	from: c.req.query("from"),
	// 	// 	to: c.req.query("to"),
	// 	// 	dir: c.req.query("dir") as "f" | "b",
	// 	// });
	// 	// return c.json(roles, 200);
	// });

	// const MAX_SIZE = 1024 * 1024 * 16;

	// app.put(
	//   "/api/media/:upload_id",
	  // bodyLimit({
	  //   maxSize: MAX_SIZE,
	  //   onError: (c) => {
	  //     return c.json({ error: "too big :(" }, 413);
	  //   },
	  // }),
	  // async (c) => {
  	//   const f = await Deno.makeTempFile({ prefix: "chat_" });
  	//   const b = await c.req.blob();
  	//   const [s0, s1] = b.stream().tee();
  	//   await Promise.all([
   //  	  blobs.putObject("media/test", s0),
   //  	  Deno.writeFile(f, s1),
  	//   ]);
  	//   return new Response(null, { status: 204 });
  	// });

  	app.post("/api/v1/_temp_media/upload", auth({ strict: true }), async (c) => {
  		const user_id = c.get("user_id");
  		const body = await c.req.formData();
  		const file = body.get("file");
  		if (!(file instanceof File)) return c.json({ error: "bad request" }, 400);
  		const media_id = uuidv7();
  		const p = await Deno.makeTempFile({ prefix: "chat_media_ "});
  		await Deno.writeFile(p, file.stream());
  		const [meta, mime] = await Promise.all([getMetadata(p), getMimeType(p)]);
  		await Deno.remove(p);
  		console.log(meta);
  		data.mediaInsert(user_id, {
				alt: body.get("alt")?.toString() ?? null,
				id: media_id,
				filename: file.name,
				url: media_id,
				source_url: null,
				thumbnail_url: null,
				mime: mime,
				size: file.size,
				height: meta.height,
				width: meta.width,
				duration: meta.duration ? Math.floor(meta.duration) : null,
			});
			await blobs.putObject(media_id, file.stream());
			return c.json({ media_id }, 200);
  	});
}

async function getMetadata(file: string) {
	const cmd = new Deno.Command("ffprobe", {
		args: ["-v" , "quiet", "-of", "json", "-show_format", "-show_streams", "-i", file],
	});
	const out = await cmd.output();
	const dec = new TextDecoder();
	const json = JSON.parse(dec.decode(out.stdout));
	const duration = parseFloat(json.format?.duration) * 1000;
	const dims = json.streams?.find((i: any) => i.disposition.default && i.width) ?? json.streams?.find((i: any) => i.width);
	return {
		width: dims?.width ?? null,
		height: dims?.height ?? null,
		duration: isNaN(duration) ? null : duration,
	}
}

async function getMimeType(file: string) {
	const cmd = new Deno.Command("file", {
		args: ["-ib", file],
	});
	const out = await cmd.output();
	const dec = new TextDecoder();
	return dec.decode(out.stdout).trim();
}
