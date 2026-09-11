//! Verified low-rank/LoRA factorization of an already compiled weight delta.
//!
//! This module does not convert an activation steering vector into weights.
//! It delegates numerical factorization to TIDE-X's canonical verified
//! low-rank core and preserves an exact identity link to the dense delta.

use crate::cross_model::models::{sha256_hex, CapabilityEvidenceKind, CapabilityMetadata, Tensor};
use crate::materialization::low_rank_shadow_materializer::{
    factor_dense_delta_verified, LowRankShadowPolicy, VerifiedLowRankFactors,
};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoRAConfig {
    pub target_module: String,
    pub target_layer: usize,
    pub policy: LowRankShadowPolicy,
}

impl LoRAConfig {
    fn validate(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.target_module.trim().is_empty() || self.target_module.len() > 4096 {
            return Err("lora_target_module_invalid".into());
        }
        self.policy.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoRAWeights {
    pub target_module: String,
    pub target_layer: usize,
    pub rank: usize,
    pub alpha: f64,
    pub a: Tensor,
    pub b: Tensor,
    pub dense_delta_sha256: String,
    pub factorization_policy_sha256: String,
}

impl LoRAWeights {
    pub fn validate(&self) -> Result<(), String> {
        self.a.validate()?;
        self.b.validate()?;
        if self.target_module.trim().is_empty()
            || self.rank == 0
            || !self.alpha.is_finite()
            || self.alpha <= 0.0
            || self.a.shape.len() != 2
            || self.b.shape.len() != 2
            || self.a.shape[0] != self.rank
            || self.b.shape[1] != self.rank
            || self.a.layer_index != self.target_layer
            || self.b.layer_index != self.target_layer
            || self.alpha != self.rank as f64
            || self.dense_delta_sha256.len() != 64
            || self.factorization_policy_sha256.len() != 64
        {
            return Err("lora_weights_invalid".into());
        }
        Ok(())
    }

    pub fn materialize_dense(&self) -> Result<Vec<f64>, String> {
        self.validate()?;
        let rows = self.b.shape[0];
        let columns = self.a.shape[1];
        let mut dense = vec![
            0.0;
            rows.checked_mul(columns)
                .ok_or("lora_dense_shape_overflow")?
        ];
        let scale = self.alpha / self.rank as f64;
        for row in 0..rows {
            for column in 0..columns {
                let mut value = 0.0;
                for component in 0..self.rank {
                    value += self.b.data[row * self.rank + component]
                        * self.a.data[component * columns + column];
                }
                dense[row * columns + column] = value * scale;
            }
        }
        Ok(dense)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoRASynthesisResult {
    pub schema: String,
    pub capability: CapabilityMetadata,
    pub lora_weights: LoRAWeights,
    pub absolute_reconstruction_error: f64,
    pub relative_reconstruction_error: f64,
    pub parameter_reduction_ratio: f64,
    pub evidence_sha256: String,
}

impl LoRASynthesisResult {
    pub fn validate(&self) -> Result<(), String> {
        self.capability.validate()?;
        self.lora_weights.validate()?;
        if self.schema != "cerebro.cross_model.verified_lora_factorization/v1"
            || self.capability.evidence_kind != CapabilityEvidenceKind::WeightDelta
            || !self.absolute_reconstruction_error.is_finite()
            || self.absolute_reconstruction_error < 0.0
            || !self.relative_reconstruction_error.is_finite()
            || self.relative_reconstruction_error < 0.0
            || !self.parameter_reduction_ratio.is_finite()
            || !(0.0..1.0).contains(&self.parameter_reduction_ratio)
        {
            return Err("lora_synthesis_result_invalid".into());
        }
        if synthesis_digest(self)? != self.evidence_sha256 {
            return Err("lora_synthesis_digest_mismatch".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoRASynthesizer;

pub type LoRASynthesizerConfig = LowRankShadowPolicy;

impl LoRASynthesizer {
    pub fn new(_config: LoRASynthesizerConfig) -> Self {
        Self
    }

    pub fn synthesize(
        &self,
        dense_delta: &Tensor,
        capability: CapabilityMetadata,
        config: LoRAConfig,
    ) -> Result<LoRASynthesisResult, Box<dyn Error + Send + Sync>> {
        dense_delta.validate()?;
        capability.validate()?;
        config.validate()?;
        if capability.evidence_kind != CapabilityEvidenceKind::WeightDelta {
            return Err("lora_requires_weight_delta_evidence".into());
        }
        if dense_delta.shape.len() != 2 || dense_delta.layer_index != config.target_layer {
            return Err("lora_dense_delta_must_be_bound_matrix".into());
        }
        let dense_delta_sha256 = sha256_hex(&serde_json::to_vec(dense_delta)?);
        if capability.evidence_sha256 != dense_delta_sha256 {
            return Err("lora_capability_delta_digest_mismatch".into());
        }
        let factors = factor_dense_delta_verified(
            dense_delta.shape[0],
            dense_delta.shape[1],
            &dense_delta.data,
            &config.policy,
        )?;
        self.build_from_verified_factors(dense_delta, capability, config, factors)
    }

    fn build_from_verified_factors(
        &self,
        dense_delta: &Tensor,
        capability: CapabilityMetadata,
        config: LoRAConfig,
        factors: VerifiedLowRankFactors,
    ) -> Result<LoRASynthesisResult, Box<dyn Error + Send + Sync>> {
        let reconstructed = factors.materialize_dense()?;
        if reconstructed.len() != dense_delta.data.len() {
            return Err("lora_core_reconstruction_shape_mismatch".into());
        }
        let policy_sha256 = config.policy.digest()?.to_string();
        // Core factors are dense = left @ right. PEFT uses B @ A * alpha/rank.
        // alpha=rank therefore preserves the verified core factorization exactly.
        let weights = LoRAWeights {
            target_module: config.target_module,
            target_layer: config.target_layer,
            rank: factors.rank,
            alpha: factors.rank as f64,
            a: Tensor::new(
                factors.right.clone(),
                vec![factors.rank, factors.columns],
                dense_delta.device,
                dense_delta.dtype,
                config.target_layer,
            ),
            b: Tensor::new(
                factors.left.clone(),
                vec![factors.rows, factors.rank],
                dense_delta.device,
                dense_delta.dtype,
                config.target_layer,
            ),
            dense_delta_sha256: sha256_hex(&serde_json::to_vec(dense_delta)?),
            factorization_policy_sha256: policy_sha256,
        };
        weights.validate()?;
        let rematerialized = weights.materialize_dense()?;
        let max_error = rematerialized
            .iter()
            .zip(&reconstructed)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max);
        if max_error
            > f64::EPSILON
                * 64.0
                * reconstructed
                    .iter()
                    .map(|v| v.abs())
                    .fold(1.0_f64, f64::max)
        {
            return Err("lora_peft_scaling_reconstruction_mismatch".into());
        }
        let mut result = LoRASynthesisResult {
            schema: "cerebro.cross_model.verified_lora_factorization/v1".into(),
            capability,
            lora_weights: weights,
            absolute_reconstruction_error: factors.absolute_reconstruction_error,
            relative_reconstruction_error: factors.relative_reconstruction_error,
            parameter_reduction_ratio: factors.parameter_reduction_ratio,
            evidence_sha256: String::new(),
        };
        result.evidence_sha256 = synthesis_digest(&result)?;
        result.validate()?;
        Ok(result)
    }

    pub fn export_manifest_json(
        &self,
        result: &LoRASynthesisResult,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        result.validate()?;
        Ok(serde_json::to_string_pretty(result)?)
    }
}

fn synthesis_digest(result: &LoRASynthesisResult) -> Result<String, String> {
    let mut unsigned = result.clone();
    unsigned.evidence_sha256.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("lora_synthesis_serialize:{error}"))
}
