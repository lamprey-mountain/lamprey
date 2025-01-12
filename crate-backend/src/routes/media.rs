use axum::{body::Body, extract::{Path, Request, State}, http::{HeaderMap, HeaderName, StatusCode}, routing::{patch, post}, Json, Router};
use tokio::io::AsyncSeekExt;
use url::Url;
use utoipa_axum::router::OpenApiRouter;

use crate::{error::Error, types::{Media, MediaCreate, MediaCreated, MediaId, MediaUpload}, ServerState};

use super::util::Auth;

const MAX_SIZE: u64 = 1024 * 1024 * 16;

/// Create a new url to upload media to. Use the media upload endpoint for actually uploading media. Media not referenced/used in other api calls will be removed after a period of time.
#[utoipa::path(
    post,
    path = "/media",
    tags = ["media"],
    responses(
        (status = StatusCode::CREATED, description = "Get room success", body = MediaCreated)
    )
)]
pub async fn media_create(
    Auth(session): Auth,
    State(state): State<ServerState>,
    Json(r): Json<MediaCreate>,
) -> Result<(StatusCode, HeaderMap, Json<MediaCreated>), Error> {
    if r.size.is_some_and(|s| s > MAX_SIZE) {
        return Err(Error::TooBig)
    }
    
    use async_tempfile::TempFile;
    let user_id = session.user_id;
    let media_id = MediaId(uuid::Uuid::now_v7());
    let temp_file = TempFile::new().await.expect("failed to create temp file!");
	let upload_url = Some(Url::parse(&format!("https://chat.celery.eu.org/api/v1/media/{media_id}")).expect("somehow constructed invalid url"));
    state.uploads.insert(media_id.clone(), MediaUpload {
        create: r.clone(),
    	user_id,
    	temp_file,
    });
    let res = MediaCreated {
    	media_id,
		upload_url,
    };
    let mut res_headers = HeaderMap::new();
    res_headers.insert("upload-length", 0.into());
    if let Some(s) = r.size {
        res_headers.insert("upload-offset", s.into());
    }
    Ok((StatusCode::CREATED, res_headers, Json(res)))
}

/// Upload media
#[utoipa::path(
    patch,
    path = "/media/{id}",
    tags = ["media"],
    params(("id", description = "Room id")),
    responses(
        (status = NO_CONTENT, description = "Upload success" )
    )
)]
pub async fn media_upload(
    Path((media_id,)): Path<(MediaId,)>,
    Auth(session): Auth,
    State(state): State<ServerState>,
    headers: HeaderMap,
    body: Request,
) -> Result<(StatusCode, HeaderMap, Json<Option<Media>>), Error> {
    let mut up = state.uploads.get_mut(&media_id).ok_or(Error::NotFound)?;
    if up.user_id != session.user_id {
        return Err(Error::NotFound);
    }
    let stat = up.temp_file.metadata().await?;
    let current_size = stat.len();
    let current_off: u64 = headers.get("upload-offset").ok_or(Error::BadHeader)?.to_str()?.parse()?;
	if current_size != current_off {
	    return Err(Error::CantOverwrite);
	}
	if up.create.size.is_some_and(|s| current_size + current_off > s) {
	    return Err(Error::TooBig);
	}
	up.temp_file.seek(std::io::SeekFrom::End(0)).await?;
	use futures_util::stream::StreamExt;
	// body.into_body().into_data_stream().fold(up.temp_file, |f, buf| {
 //        tokio::io::copy(&mut buf?, &mut f)
 //    });
    // up.temp_file
	// body.into_body().into_data_stream().forward(&mut f).await;
// 		const blob = await c.req.blob();
// 		await blob.stream().pipeTo(f.writable);
// 		const end_size = (await Deno.stat(up.temp_file)).size;
// 		if (end_size > up.size) {
// 			await Deno.remove(up.temp_file);
// 			locks.delete(media_id);
// 			return c.json({ error: "too big :(" }, 413);
// 		} else if (end_size === up.size) {
// 			using f = await Deno.open(up.temp_file, { read: true });
// 			await blobs.putObject(media_id, f.readable);
//   		const [meta, mime] = await Promise.all([getMetadata(up.temp_file), getMimeType(up.temp_file)]);
//   		console.log(meta);
//   		const media = await data.mediaInsert(user_id, {
// 				alt: up.alt ?? null,
// 				id: media_id,
// 				filename: up.filename,
// 				url: media_id,
// 				source_url: null,
// 				thumbnail_url: null,
// 				mime: mime,
// 				size: up.size,
// 				height: meta.height,
// 				width: meta.width,
// 				duration: meta.duration ? Math.floor(meta.duration) : null,
// 			});
// 			await Deno.remove(up.temp_file);
// 			locks.delete(media_id);
// 			media.url = await blobs.presignedGetUrl(media.url);
// 			return c.json(media, 200, {
// 				"Upload-Offset": stat.size.toString(),
// 				"Upload-Length": up.size.toString(),
// 			});
// 		} else {
// 			f.close();
// 			locks.delete(media_id);
// 			return new Response(null, {
// 				status: 204,
// 				headers: {
// 					"Upload-Offset": end_size.toString(),
// 					"Upload-Length": up.size.toString(),
// 				},
// 			});
// 		}
// 	});
    Ok(todo!())
}

// 	app.openAPIRegistry.registerPath(MediaCheck);

// 	app.openapi(withAuth(MediaGet), async (c) => {
// 		const user_id = c.get("user_id");
// 		const media_id = c.req.param("media_id");
// 		// extremely dubious
// 		if (c.req.method === "HEAD") {
// 			const up = uploads.get(media_id);
// 			console.log({ uploads })
// 			if (!up) return c.json({ error: "not found" }, 404);
// 			if (up.user_id !== user_id) return c.json({ error: "not found" }, 404);
// 			const stat = await Deno.stat(up.temp_file);
// 			return new Response(null, {
// 				status: 204,
// 				headers: {
// 					"Upload-Offset": stat.size.toString(),
// 					"Upload-Length": up.size.toString(),
// 				},
// 			}) as any;
// 		} else {
// 			const media = await data.mediaSelect(media_id);
// 			if (!media) return c.json({ error: "not found" }, 404);
// 			media.url = await blobs.presignedGetUrl(media.url);
// 			return c.json(media, 200);
// 		}
// 	});

// async function getMetadata(file: string) {
// 	const cmd = new Deno.Command("ffprobe", {
// 		args: ["-v" , "quiet", "-of", "json", "-show_format", "-show_streams", "-i", file],
// 	});
// 	const out = await cmd.output();
// 	const dec = new TextDecoder();
// 	const json = JSON.parse(dec.decode(out.stdout));
// 	const duration = parseFloat(json.format?.duration) * 1000;
// 	const dims = json.streams?.find((i: any) => i.disposition.default && i.width) ?? json.streams?.find((i: any) => i.width);
// 	return {
// 		width: dims?.width ?? null,
// 		height: dims?.height ?? null,
// 		duration: isNaN(duration) ? null : duration,
// 	}
// }

// async function getMimeType(file: string) {
// 	const cmd = new Deno.Command("file", {
// 		args: ["-ib", file],
// 	});
// 	const out = await cmd.output();
// 	const dec = new TextDecoder();
// 	return dec.decode(out.stdout).trim();
// }

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(media_create, media_upload))
}
