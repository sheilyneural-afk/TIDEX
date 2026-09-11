use cerebro_tidex::error::{BrainError, BrainResult};
use cerebro_tidex::shadow_evaluation::{evaluate_shadow_bundle_payloads, ShadowEvaluationBundle};
use std::io::{Read, Write};
use std::path::Path;

const RUNNER_INPUT_PATH: &str = "/tidex/input";
const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;

fn run() -> BrainResult<()> {
    let input_path = std::env::var_os("TIDEX_INPUT_PATH")
        .ok_or_else(|| BrainError::Invalid("tidex_input_path_missing".into()))?;
    if Path::new(&input_path) != Path::new(RUNNER_INPUT_PATH) {
        return Err(BrainError::Invalid("tidex_input_path_invalid".into()));
    }
    let file = std::fs::File::open(&input_path)?;
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() || metadata.len() == 0 || metadata.len() > MAX_INPUT_BYTES {
        return Err(BrainError::Invalid(
            "shadow_runner_input_file_invalid".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(BrainError::Invalid("shadow_runner_bundle_too_large".into()));
    }
    let bundle: ShadowEvaluationBundle = serde_json::from_slice(&bytes)
        .map_err(|_| BrainError::Invalid("shadow_runner_bundle_encoding_invalid".into()))?;
    if serde_json::to_vec(&bundle)? != bytes {
        return Err(BrainError::Invalid(
            "shadow_runner_bundle_not_canonical".into(),
        ));
    }
    let output = evaluate_shadow_bundle_payloads(&bundle)?;
    let encoded = serde_json::to_vec(&output)?;
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&encoded)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
