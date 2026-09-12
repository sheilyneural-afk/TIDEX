//! Plasticity module
//!
//! Numerical control-plane controllers (BCM, eligibility, neuromodulation,
//! routing, content, PI, ELO). These are advisory metaplasticity controllers —
//! not the cross-model discovery/intervention orchestrator in
//! `plasticity_engine` / `plasticity_daemon`.

pub mod bcm_metaplasticity;
pub mod content_plasticity;
pub mod eligibility_traces;
pub mod elo_system;
pub mod neuromodulation;
pub mod pi_controller;
pub mod routing_plasticity;

pub use bcm_metaplasticity::{BCMConfig, BCMMetaplasticity, BCMState};
pub use content_plasticity::{
    ContentPlasticity, ContentPlasticityConfig, ContentPlasticityMatrix, ContentPlasticityState,
};
pub use eligibility_traces::{EligibilityTrace, EligibilityTraceConfig, EligibilityTraces};
pub use elo_system::{ELOConfig, ELOState, ELOSystem};
pub use neuromodulation::{
    Neuromodulation, NeuromodulationConfig, NeuromodulationSignal, Neuromodulator,
};
pub use pi_controller::{PIController, PIControllerConfig, PIControllerState};
pub use routing_plasticity::{
    RoutingDecision, RoutingObservation, RoutingPlasticity, RoutingPlasticityConfig,
    RoutingPlasticityMatrix, RoutingStatistics,
};

use std::collections::BTreeMap;
use std::path::Path;

/// Bundled validated configs loaded from `config/plasticity.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlasticityControlPlaneConfigs {
    pub bcm: BCMConfig,
    pub eligibility: EligibilityTraceConfig,
    pub neuromodulation: NeuromodulationConfig,
    pub routing: RoutingPlasticityConfig,
    pub content: ContentPlasticityConfig,
    pub pi: PIControllerConfig,
    pub elo: ELOConfig,
}

const CONTROL_PLANE_SCHEMA: &str = "tidex.cross_model.control_plane/v1";

/// Fail-closed loader for the repo control-plane TOML. Unknown keys/sections,
/// invalid schema, or config validation failure all error.
pub fn load_plasticity_control_plane_configs(
    path: &Path,
) -> Result<(PlasticityControlPlaneConfigs, Vec<u8>), String> {
    let bytes = std::fs::read(path).map_err(|_| "plasticity_toml_unreadable".to_string())?;
    if bytes.is_empty() || bytes.len() > 1_048_576 {
        return Err("plasticity_toml_size_invalid".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "plasticity_toml_not_utf8".to_string())?;
    let configs = parse_plasticity_control_plane_toml(text)?;
    Ok((configs, bytes))
}

pub fn default_plasticity_toml_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("config/plasticity.toml")
}

pub fn parse_plasticity_control_plane_toml(
    text: &str,
) -> Result<PlasticityControlPlaneConfigs, String> {
    let mut schema: Option<String> = None;
    let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut current: Option<String> = None;

    for (line_no, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            let name = rest
                .strip_suffix(']')
                .ok_or_else(|| format!("plasticity_toml_section_syntax:{}", line_no + 1))?
                .trim();
            if name.is_empty()
                || !matches!(
                    name,
                    "bcm"
                        | "eligibility"
                        | "modulation"
                        | "routing"
                        | "content_drift"
                        | "pi"
                        | "elo"
                )
            {
                return Err(format!("plasticity_toml_unknown_section:{name}"));
            }
            if sections.contains_key(name) {
                return Err(format!("plasticity_toml_duplicate_section:{name}"));
            }
            sections.insert(name.to_string(), BTreeMap::new());
            current = Some(name.to_string());
            continue;
        }
        let (key, value) = split_toml_assignment(line)
            .ok_or_else(|| format!("plasticity_toml_assignment_syntax:{}", line_no + 1))?;
        if let Some(section) = current.as_ref() {
            let slot = sections.get_mut(section).expect("section inserted");
            if slot.insert(key.to_string(), value.to_string()).is_some() {
                return Err(format!("plasticity_toml_duplicate_key:{section}.{key}"));
            }
        } else if key == "schema" {
            if schema.is_some() {
                return Err("plasticity_toml_duplicate_schema".into());
            }
            schema = Some(parse_toml_string(value)?);
        } else {
            return Err(format!("plasticity_toml_unknown_top_level:{key}"));
        }
    }

    if schema.as_deref() != Some(CONTROL_PLANE_SCHEMA) {
        return Err("plasticity_toml_schema_invalid".into());
    }

    let bcm = BCMConfig {
        initial_theta: required_f64(&sections, "bcm", "initial_theta")?,
        window_size: required_usize(&sections, "bcm", "window_size")?,
        learning_rate: required_f64(&sections, "bcm", "learning_rate")?,
        theta_decay: required_f64(&sections, "bcm", "theta_decay")?,
    };
    expect_exact_keys(
        &sections,
        "bcm",
        &[
            "initial_theta",
            "window_size",
            "learning_rate",
            "theta_decay",
        ],
    )?;

    let eligibility = EligibilityTraceConfig {
        initial_trace: required_f64(&sections, "eligibility", "initial_trace")?,
        decay_factor: required_f64(&sections, "eligibility", "decay_factor")?,
        trace_update_rate: required_f64(&sections, "eligibility", "trace_update_rate")?,
        max_trace_value: required_f64(&sections, "eligibility", "max_trace_value")?,
    };
    expect_exact_keys(
        &sections,
        "eligibility",
        &[
            "initial_trace",
            "decay_factor",
            "trace_update_rate",
            "max_trace_value",
        ],
    )?;

    let mut weights = BTreeMap::new();
    weights.insert(Neuromodulator::Reward, required_f64(&sections, "modulation", "reward_weight")?);
    weights.insert(
        Neuromodulator::Attention,
        required_f64(&sections, "modulation", "attention_weight")?,
    );
    weights.insert(
        Neuromodulator::Novelty,
        required_f64(&sections, "modulation", "novelty_weight")?,
    );
    weights.insert(
        Neuromodulator::Stability,
        required_f64(&sections, "modulation", "stability_weight")?,
    );
    let neuromodulation = NeuromodulationConfig {
        weights,
        decay_factor: required_f64(&sections, "modulation", "decay_factor")?,
    };
    expect_exact_keys(
        &sections,
        "modulation",
        &[
            "reward_weight",
            "attention_weight",
            "novelty_weight",
            "stability_weight",
            "decay_factor",
        ],
    )?;

    let routing = RoutingPlasticityConfig {
        uncertainty_weight: required_f64(&sections, "routing", "uncertainty_weight")?,
        minimum_measured_score: required_f64(&sections, "routing", "minimum_measured_score")?,
    };
    expect_exact_keys(&sections, "routing", &["uncertainty_weight", "minimum_measured_score"])?;

    let content = ContentPlasticityConfig {
        similarity_threshold: required_f64(&sections, "content_drift", "similarity_threshold")?,
        adaptation_rate: required_f64(&sections, "content_drift", "adaptation_rate")?,
        maximum_pressure: required_f64(&sections, "content_drift", "maximum_pressure")?,
    };
    expect_exact_keys(
        &sections,
        "content_drift",
        &[
            "similarity_threshold",
            "adaptation_rate",
            "maximum_pressure",
        ],
    )?;

    let pi = PIControllerConfig {
        proportional_gain: required_f64(&sections, "pi", "proportional_gain")?,
        integral_gain: required_f64(&sections, "pi", "integral_gain")?,
        output_min: required_f64(&sections, "pi", "output_min")?,
        output_max: required_f64(&sections, "pi", "output_max")?,
        integral_windup_limit: required_f64(&sections, "pi", "integral_windup_limit")?,
    };
    expect_exact_keys(
        &sections,
        "pi",
        &[
            "proportional_gain",
            "integral_gain",
            "output_min",
            "output_max",
            "integral_windup_limit",
        ],
    )?;

    let elo = ELOConfig {
        initial_rating: required_f64(&sections, "elo", "initial_rating")?,
        k_factor: required_f64(&sections, "elo", "k_factor")?,
        rating_floor: required_f64(&sections, "elo", "rating_floor")?,
        rating_ceiling: required_f64(&sections, "elo", "rating_ceiling")?,
        logistic_scale: required_f64(&sections, "elo", "logistic_scale")?,
    };
    expect_exact_keys(
        &sections,
        "elo",
        &[
            "initial_rating",
            "k_factor",
            "rating_floor",
            "rating_ceiling",
            "logistic_scale",
        ],
    )?;

    // Validate by constructing controllers (same fail-closed path as runtime).
    let _ = BCMMetaplasticity::new(bcm.clone())?;
    let _ = EligibilityTraces::new(eligibility.clone())?;
    let _ = Neuromodulation::new(neuromodulation.clone())?;
    let _ = RoutingPlasticity::new(routing.clone())?;
    let _ = ContentPlasticity::new(content.clone())?;
    let _ = PIController::new(pi.clone())?;
    let _ = ELOSystem::new(elo.clone())?;

    Ok(PlasticityControlPlaneConfigs {
        bcm,
        eligibility,
        neuromodulation,
        routing,
        content,
        pi,
        elo,
    })
}

fn split_toml_assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }
    Some((key, value))
}

fn parse_toml_string(value: &str) -> Result<String, String> {
    let value = value.trim();
    let inner = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or_else(|| "plasticity_toml_string_invalid".to_string())?;
    if inner.contains('"') || inner.contains('\\') {
        return Err("plasticity_toml_string_escapes_unsupported".into());
    }
    Ok(inner.to_string())
}

fn required_section<'a>(
    sections: &'a BTreeMap<String, BTreeMap<String, String>>,
    name: &str,
) -> Result<&'a BTreeMap<String, String>, String> {
    sections
        .get(name)
        .ok_or_else(|| format!("plasticity_toml_missing_section:{name}"))
}

fn required_f64(
    sections: &BTreeMap<String, BTreeMap<String, String>>,
    section: &str,
    key: &str,
) -> Result<f64, String> {
    let raw = required_section(sections, section)?
        .get(key)
        .ok_or_else(|| format!("plasticity_toml_missing_key:{section}.{key}"))?;
    let value: f64 = raw
        .parse()
        .map_err(|_| format!("plasticity_toml_f64_invalid:{section}.{key}"))?;
    if !value.is_finite() {
        return Err(format!("plasticity_toml_f64_invalid:{section}.{key}"));
    }
    Ok(value)
}

fn required_usize(
    sections: &BTreeMap<String, BTreeMap<String, String>>,
    section: &str,
    key: &str,
) -> Result<usize, String> {
    let raw = required_section(sections, section)?
        .get(key)
        .ok_or_else(|| format!("plasticity_toml_missing_key:{section}.{key}"))?;
    raw.parse()
        .map_err(|_| format!("plasticity_toml_usize_invalid:{section}.{key}"))
}

fn expect_exact_keys(
    sections: &BTreeMap<String, BTreeMap<String, String>>,
    section: &str,
    expected: &[&str],
) -> Result<(), String> {
    let keys = required_section(sections, section)?;
    if keys.len() != expected.len() {
        return Err(format!("plasticity_toml_unexpected_keys:{section}"));
    }
    for key in expected {
        if !keys.contains_key(*key) {
            return Err(format!("plasticity_toml_missing_key:{section}.{key}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plasticity_toml_loader_accepts_repo_control_plane_file() {
        let (configs, bytes) =
            load_plasticity_control_plane_configs(&default_plasticity_toml_path()).unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(configs.bcm.learning_rate, 0.01);
        assert_eq!(configs.elo.k_factor, 32.0);
        assert_eq!(configs.neuromodulation.weights[&Neuromodulator::Reward], 0.4);
    }

    #[test]
    fn plasticity_toml_loader_rejects_unknown_section() {
        let text = r#"
schema = "tidex.cross_model.control_plane/v1"
[bcm]
initial_theta = 0.5
window_size = 100
learning_rate = 0.01
theta_decay = 0.001
[mystery]
x = 1
"#;
        assert!(parse_plasticity_control_plane_toml(text).is_err());
    }
}
