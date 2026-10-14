// reference code

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use image::{ImageReader, imageops::FilterType};
use ndarray::Array4;
use ort::{session::Session, value::Tensor};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

struct Shared {
    // https://docs.rs/ort/latest/ort/session/struct.Session.html#why-does-run-take-mut-self
    // > If you need concurrent inference, we recommend creating one session per thread or batching multiple requests at once
    session: Mutex<Session>,
}

#[tokio::main]
async fn main() {
    let session = Session::builder()
        .unwrap()
        // .with_optimization_level(opt_level)
        // .with_intra_threads(4)
        .commit_from_file("models/model_q4.onnx")
        .unwrap();

    {
        let meta = session.metadata().unwrap();
        dbg!(meta.name());
        dbg!(meta.description());
        dbg!(meta.producer());
        dbg!(meta.version());
    }

    let state = Shared {
        session: Mutex::new(session),
    };

    let router = axum::Router::new()
        .route("/", axum::routing::get(|| async { "hello, world!" }))
        .route("/health", axum::routing::get(health))
        .route("/scan", axum::routing::post(scan_image))
        .with_state(Arc::new(state));

    let listener = TcpListener::bind("0.0.0.0:4100").await.unwrap();
    // let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

#[derive(Debug, Serialize)]
struct Health {
    ok: bool,
}

#[derive(Debug, Deserialize)]
struct ScanRequest {
    path: PathBuf,
}

#[derive(Debug, Serialize)]
struct ScanResponse {
    score: f32,
}

async fn health() -> impl IntoResponse {
    println!("-> health");
    Json(Health { ok: true })
}

#[derive(Debug, thiserror::Error)]
enum ScanImageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Tensor error: {0}")]
    Tensor(#[from] ort::Error),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ScanImageError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ScanImageError::Io(err) if err.kind() == std::io::ErrorKind::NotFound => {
                (StatusCode::NOT_FOUND, err.to_string())
            }
            ScanImageError::Io(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ScanImageError::Image(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ScanImageError::Tensor(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ScanImageError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, message).into_response()
    }
}

async fn scan_image(
    State(s): State<Arc<Shared>>,
    Json(req): Json<ScanRequest>,
) -> Result<Json<ScanResponse>, ScanImageError> {
    println!("-> scan");
    let score = tokio::task::spawn_blocking(move || run(&s, &req.path))
        .await
        .map_err(|e| ScanImageError::Internal(e.to_string()))??;
    Ok(Json(ScanResponse { score }))
}

fn run(shared: &Shared, path: &Path) -> Result<f32, ScanImageError> {
    // load image
    let img = ImageReader::open(path)?.with_guessed_format()?.decode()?;

    // resize and normalize image (each pixel is mapped from 0..255 to -1..1)
    let rgb = img.resize_exact(224, 224, FilterType::Triangle).to_rgb8();
    let mut input = Array4::<f32>::zeros((1, 3, 224, 224)); // (batch size, channels, height, width)
    for (x, y, p) in rgb.enumerate_pixels() {
        for c in 0..3 {
            input[[0, c, y as usize, x as usize]] = (p[c] as f32 / 255.0 - 0.5) / 0.5;
        }
    }

    let tensor = Tensor::from_array(input)?;

    // run the model
    let mut session = shared.session.lock().unwrap();
    let outputs = session.run(ort::inputs!["pixel_values" => tensor])?;
    let logits = outputs["logits"].try_extract_array::<f32>()?;

    // softmax logits
    let (normal, nsfw) = (logits[[0, 0]], logits[[0, 1]]);
    Ok(1.0 / (1.0 + (normal - nsfw).exp()))
}


