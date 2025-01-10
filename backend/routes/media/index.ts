import { OpenAPIHono, z } from "@hono/zod-openapi";
import { auth, withAuth, withMiddleware } from "../auth.ts";
import { blobs, data, events, HonoEnv } from "globals";
import { uuidv7 } from "uuidv7";
import { Room, MediaCreateBody } from "../../types.ts";
import { UUID_MAX, UUID_MIN } from "../../util.ts";
import { MediaCheck, MediaCreate, MediaGet, MediaUpload } from "./def.ts";
import { bodyLimit } from "hono/body-limit";

type MediaCreateBodyT = z.infer<typeof MediaCreateBody>;

type PartialUpload = MediaCreateBodyT & {
	user_id: string,
	id: string,
	temp_file: string,
}

const MAX_SIZE = 1024 * 1024 * 16;
const uploads = new Map<string, PartialUpload>();
const locks = new Set<string>();
const tmp = await Deno.makeTempDir({ prefix: "chat_media_" });
Deno.addSignalListener("SIGTERM", () => Deno.removeSync(tmp, { recursive: true }));

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(MediaCreate), async (c) => {
	  const r = await c.req.json();
	  if (r.size > MAX_SIZE) return c.json({ error: "too big :(" }, 413);
	  const user_id = c.get("user_id");
	  const media_id = uuidv7();
		// const temp_file = await Deno.makeTempFile({ prefix: "chat_media_ "});
	  const temp_file = `${tmp}/${media_id}`;
	  uploads.set(media_id, { ...r, id: media_id, user_id, temp_file })
		return c.json({
			media_id,
			upload_url: `https://chat.celery.eu.org/api/v1/media/${media_id}`
		}, 201, {
			"Upload-Offset": r.size.toString(),
			"Upload-Length": "0",
		});
	});

	app.openapi(withMiddleware(MediaUpload,
		auth({ strict: false }), 
	  bodyLimit({
			// TODO: max size based on current req size
	    maxSize: MAX_SIZE,
	    onError: (c) => {
	      return c.json({ error: "too big :(" }, 413);
	    },
	  })), async (c) => {
		const user_id = c.get("user_id");
		const media_id = c.req.param("media_id");
		const up = uploads.get(media_id);
		if (!up) return c.json({ error: "not found" }, 404);
		if (up.user_id !== user_id) return c.json({ error: "not found" }, 404);
		if (locks.has(media_id)) return c.json({ error: "already uploading!" }, 409);
		locks.add(media_id);
		const f = await Deno.open(up.temp_file, { create: true, append: true });
		const stat = await f.stat();
		const current_size = stat.size;
		const current_off = parseInt(c.req.header("Upload-Offset")!, 10);
		if (isNaN(current_off)) return c.json({ error: "bad offset" }, 409);
		if (stat.size !== current_off) {
			return c.json({ error: "can't overwrite already uploaded data!" }, 409);
		}
		if (current_off + current_size > up.size) {
			return c.json({ error: "too big :(" }, 413);
		}
		const blob = await c.req.blob();
		await blob.stream().pipeTo(f.writable);
		const end_size = (await Deno.stat(up.temp_file)).size;
		if (end_size > up.size) {
			await Deno.remove(up.temp_file);
			locks.delete(media_id);
			return c.json({ error: "too big :(" }, 413);
		} else if (end_size === up.size) {
			using f = await Deno.open(up.temp_file, { read: true });
			await blobs.putObject(media_id, f.readable);
  		const [meta, mime] = await Promise.all([getMetadata(up.temp_file), getMimeType(up.temp_file)]);
  		console.log(meta);
  		const media = await data.mediaInsert(user_id, {
				alt: up.alt ?? null,
				id: media_id,
				filename: up.filename,
				url: media_id,
				source_url: null,
				thumbnail_url: null,
				mime: mime,
				size: up.size,
				height: meta.height,
				width: meta.width,
				duration: meta.duration ? Math.floor(meta.duration) : null,
			});
			await Deno.remove(up.temp_file);
			locks.delete(media_id);
			media.url = await blobs.presignedGetUrl(media.url);
			return c.json(media, 200, {
				"Upload-Offset": stat.size.toString(),
				"Upload-Length": up.size.toString(),
			});
		} else {
			f.close();
			locks.delete(media_id);
			return new Response(null, {
				status: 204,
				headers: {
					"Upload-Offset": end_size.toString(),
					"Upload-Length": up.size.toString(),
				},
			});
		}
	});

	app.openAPIRegistry.registerPath(MediaCheck);
	
	app.openapi(withAuth(MediaGet), async (c) => {
		const user_id = c.get("user_id");
		const media_id = c.req.param("media_id");
		// extremely dubious
		if (c.req.method === "HEAD") {
			const up = uploads.get(media_id);
			console.log({ uploads })
			if (!up) return c.json({ error: "not found" }, 404);
			if (up.user_id !== user_id) return c.json({ error: "not found" }, 404);
			const stat = await Deno.stat(up.temp_file);
			return new Response(null, {
				status: 204,
				headers: {
					"Upload-Offset": stat.size.toString(),
					"Upload-Length": up.size.toString(),
				},
			}) as any;
		} else {
			const media = await data.mediaSelect(media_id);
			if (!media) return c.json({ error: "not found" }, 404);
			media.url = await blobs.presignedGetUrl(media.url);
			return c.json(media, 200);
		}
	});

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
