use std::path::Path;

use ndarray::{Array4, s};
use ort::{session::Session, value::Tensor};
use tokio::sync::{mpsc, oneshot};

/// how many requests may be queued at once
const JOB_QUEUE_SIZE: usize = 64;

/// how many requests the worker can run in a single inference run
///
/// this should probably be configurable tbh. the `16` comes from <https://huggingface.co/Falconsai/nsfw_image_detection>.
const MAX_BATCH_SIZE: usize = 16;

// NOTE: are these constants embedded in the model?
const IMAGE_WIDTH: usize = 224;
const IMAGE_HEIGHT: usize = 224;
const IMAGE_CHANNELS: usize = 3;

/// an error that occured in the scanner
#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("Tensor error: {0}")]
    Tensor(#[from] ort::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// request to scan an image
struct Job {
    /// preprocessed image of shape (1, channels, height, width)
    input: Array4<f32>,
    respond: oneshot::Sender<Result<f32, ScannerError>>,
}

/// nsfw image scanner
pub struct Scanner {
    jobs: mpsc::Sender<Job>,
}

// NOTE; it doesnt really make *that* much sense for this to be async, since media scanning is compute bound not io bound
impl Scanner {
    pub async fn load(model_path: &Path) -> Result<Self, ScannerError> {
        let model_path = model_path.to_owned();
        let session = tokio::task::spawn_blocking(move || load_session(&model_path))
            .await
            .map_err(|e| ScannerError::Internal(e.to_string()))??;

        let (jobs, receiver) = mpsc::channel(JOB_QUEUE_SIZE);
        std::thread::Builder::new()
            .name("scanner-nsfw-worker".to_string())
            .spawn(move || worker(session, receiver))
            .map_err(|e| ScannerError::Internal(format!("failed to spawn worker: {e}")))?;

        Ok(Scanner { jobs })
    }

    /// scan a single image, returning the probability that it is nsfw
    pub async fn run(&self, path: &Path) -> Result<f32, ScannerError> {
        let path = path.to_owned();
        let input = tokio::task::spawn_blocking(move || preprocess(&path))
            .await
            .map_err(|e| ScannerError::Internal(e.to_string()))??;

        let (respond, result) = oneshot::channel();
        self.jobs
            .send(Job { input, respond })
            .await
            .map_err(|_| ScannerError::Internal("scanner worker stopped".to_string()))?;

        result
            .await
            .map_err(|_| ScannerError::Internal("scanner worker dropped request".to_string()))?
    }
}

/// load a pretrained model
fn load_session(model_path: &Path) -> Result<Session, ScannerError> {
    let environment = ort::init().with_name("scanner-nsfw").build()?;
    let session = Session::builder(&environment)?.commit_from_file(model_path)?;
    Ok(session)
}

/// decode an image and normalize it into a `(1, C, H, W)` tensor, mapping each
/// pixel from `0..255` to `-1..1`.
fn preprocess(path: &Path) -> Result<Array4<f32>, ScannerError> {
    let img = image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?;

    let rgb = img
        .resize_exact(
            IMAGE_WIDTH as u32,
            IMAGE_HEIGHT as u32,
            image::imageops::FilterType::Triangle,
        )
        .to_rgb8();

    let mut input = Array4::<f32>::zeros((1, IMAGE_CHANNELS, IMAGE_HEIGHT, IMAGE_WIDTH));
    for (x, y, p) in rgb.enumerate_pixels() {
        for c in 0..IMAGE_CHANNELS {
            input[[0, c, y as usize, x as usize]] = (p[c] as f32 / 255.0 - 0.5) / 0.5;
        }
    }

    Ok(input)
}

fn worker(mut session: Session, mut receiver: mpsc::Receiver<Job>) {
    while let Some(first) = receiver.blocking_recv() {
        let mut jobs = Vec::with_capacity(MAX_BATCH_SIZE);
        jobs.push(first);

        // opportunistically drain whatever else is already queued
        while jobs.len() < MAX_BATCH_SIZE {
            match receiver.try_recv() {
                Ok(job) => jobs.push(job),
                Err(_) => break,
            }
        }

        let batch_size = jobs.len();
        let mut batch =
            Array4::<f32>::zeros((batch_size, IMAGE_CHANNELS, IMAGE_HEIGHT, IMAGE_WIDTH));
        for (i, job) in jobs.iter().enumerate() {
            // copy job.input into the i'th image/"slot" of batch
            batch
                .slice_mut(s![i, .., .., ..])
                .assign(&job.input.slice(s![0, .., .., ..]));
        }

        match run_batch(&mut session, batch) {
            Ok(scores) => {
                for (job, score) in jobs.into_iter().zip(scores) {
                    let _ = job.respond.send(Ok(score));
                }
            }
            Err(err) => {
                // TODO: maybe make ScannerError Clone or respond with an Arc'd error
                let message = err.to_string();
                for job in jobs {
                    let _ = job
                        .respond
                        .send(Err(ScannerError::Internal(message.clone())));
                }
            }
        }
    }
}

fn run_batch(session: &mut Session, batch: Array4<f32>) -> Result<Vec<f32>, ScannerError> {
    let tensor = Tensor::from_array(batch)?;
    let outputs = session.run(ort::inputs!["pixel_values" => tensor])?;
    let logits = outputs["logits"].try_extract_array::<f32>()?;

    // softmax, get nsfw probability
    let mut scores = Vec::with_capacity(logits.shape()[0]);
    for row in logits.outer_iter() {
        let (normal, nsfw) = (row[0], row[1]);
        scores.push(1.0 / (1.0 + (normal - nsfw).exp()));
    }

    Ok(scores)
}
