use std::fs;
use std::path::{Path, PathBuf};

use tidex::materialization::activation_steering_materializer::ActivationSteeringPolicy;
use tidex::materialization::low_rank_shadow_materializer::LowRankShadowPolicy;
use tidex::materialization::materialization_selector::BackendSelectionPolicy;
use tidex::materialization::sparse_shadow_materializer::SparseShadowPolicy;

fn project_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read_json<T: serde::de::DeserializeOwned>(relative: &str) -> T {
    let path = project_file(relative);
    let bytes = fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("failed to deserialize {}: {error}", path.display()))
}

#[test]
fn production_materialization_policies_match_runtime_contracts() {
    let backend: BackendSelectionPolicy =
        read_json("config/materialization/backend-selection.json");
    backend
        .validate()
        .expect("backend selection policy must satisfy runtime contract");

    let low_rank: LowRankShadowPolicy = read_json("config/materialization/low-rank-policy.json");
    low_rank
        .validate()
        .expect("low-rank policy must satisfy runtime contract");

    let sparse: SparseShadowPolicy = read_json("config/materialization/sparse-policy.json");
    sparse
        .validate()
        .expect("sparse policy must satisfy runtime contract");

    let steering: ActivationSteeringPolicy =
        read_json("config/materialization/steering-policy.json");
    steering
        .digest()
        .expect("activation-steering policy must satisfy runtime contract");
}

#[cfg(feature = "cross-model-plasticity")]
#[test]
fn production_behavioral_evaluations_match_runtime_contracts() {
    use tidex::cross_model::discovery::BehavioralBenchmark;

    for relative in [
        "config/evaluations/binary-logic.json",
        "config/evaluations/integer-arithmetic.json",
        "config/evaluations/python-semantics.json",
    ] {
        let benchmark: BehavioralBenchmark = read_json(relative);
        benchmark.validate().unwrap_or_else(|error| {
            panic!("{relative} violates runtime benchmark contract: {error}")
        });
        benchmark.digest().unwrap_or_else(|error| {
            panic!("{relative} cannot produce authenticated digest: {error}")
        });
    }
}
