//! Calibrated cross-model activation alignment.
//!
//! Alignment is learned only from paired measured activations. No dimension
//! padding, truncation, interpolation, layer-ratio scaling, or method-name
//! substitution is permitted.

use crate::cross_model::models::{
    sha256_hex, AlignmentMethod, AlignmentResult, DType, Device, LLMModel, ModelAccess, Tensor,
};
use crate::linalg::{solve, Matrix};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ActivationPair {
    pub source: Tensor,
    pub target: Tensor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AlignmentCalibration {
    pub schema: String,
    pub source_model: String,
    pub target_model: String,
    pub source_runtime_metadata_sha256: String,
    pub target_runtime_metadata_sha256: String,
    pub source_layer: usize,
    pub target_layer: usize,
    pub ridge_lambda: f64,
    pub training_pairs: Vec<ActivationPair>,
    pub validation_pairs: Vec<ActivationPair>,
    pub calibration_sha256: String,
}

impl AlignmentCalibration {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "cerebro.cross_model.alignment_calibration/v1"
            || self.source_model.trim().is_empty()
            || self.target_model.trim().is_empty()
            || self.source_model == self.target_model
            || !self.ridge_lambda.is_finite()
            || self.ridge_lambda <= 0.0
            || self.training_pairs.len() < 2
            || self.validation_pairs.is_empty()
            || self.training_pairs.len() > 1024
            || self.validation_pairs.len() > 1024
        {
            return Err("alignment_calibration_invalid".into());
        }
        let first = self
            .training_pairs
            .first()
            .ok_or("alignment_training_empty")?;
        first.source.validate()?;
        first.target.validate()?;
        let source_dim = first.source.data.len();
        let target_dim = first.target.data.len();
        if source_dim == 0 || target_dim == 0 {
            return Err("alignment_dimension_zero".into());
        }
        for pair in self.training_pairs.iter().chain(&self.validation_pairs) {
            pair.source.validate()?;
            pair.target.validate()?;
            if pair.source.layer_index != self.source_layer
                || pair.target.layer_index != self.target_layer
                || pair.source.data.len() != source_dim
                || pair.target.data.len() != target_dim
            {
                return Err("alignment_pair_binding_invalid".into());
            }
        }
        if calibration_digest(self)? != self.calibration_sha256 {
            return Err("alignment_calibration_digest_mismatch".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CrossModelAlignerConfig {
    pub ridge_lambda: f64,
    pub maximum_validation_residual: f64,
    pub maximum_calibration_pairs: usize,
}

impl Default for CrossModelAlignerConfig {
    fn default() -> Self {
        Self {
            ridge_lambda: 1e-4,
            maximum_validation_residual: 0.35,
            maximum_calibration_pairs: 256,
        }
    }
}

impl CrossModelAlignerConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.ridge_lambda.is_finite()
            || self.ridge_lambda <= 0.0
            || !self.maximum_validation_residual.is_finite()
            || self.maximum_validation_residual < 0.0
            || self.maximum_calibration_pairs < 3
            || self.maximum_calibration_pairs > 1024
        {
            return Err("cross_model_aligner_config_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct FittedRidgeMap {
    source_layer: usize,
    target_layer: usize,
    source_training: Vec<Vec<f64>>,
    coefficients: Vec<Vec<f64>>,
    calibration_sha256: String,
    validation_residual: f64,
}

pub struct CrossModelAligner {
    config: CrossModelAlignerConfig,
}

impl CrossModelAligner {
    pub fn new(config: CrossModelAlignerConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn calibrate_from_prompts(
        &self,
        source_model: &dyn LLMModel,
        target_model: &dyn LLMModel,
        source_layer: usize,
        target_layer: usize,
        training_prompts: &[String],
        validation_prompts: &[String],
    ) -> Result<AlignmentCalibration, Box<dyn Error + Send + Sync>> {
        self.config.validate()?;
        if !source_model.supports(ModelAccess::InternalActivations)
            || !target_model.supports(ModelAccess::InternalActivations)
        {
            return Err("alignment_requires_internal_activation_backends".into());
        }
        if training_prompts.len() < 2
            || validation_prompts.is_empty()
            || training_prompts.len() + validation_prompts.len()
                > self.config.maximum_calibration_pairs
            || training_prompts
                .iter()
                .chain(validation_prompts)
                .any(|prompt| prompt.trim().is_empty())
        {
            return Err("alignment_prompt_split_invalid".into());
        }
        let collect =
            |prompts: &[String]| -> Result<Vec<ActivationPair>, Box<dyn Error + Send + Sync>> {
                prompts
                    .iter()
                    .map(|prompt| {
                        Ok(ActivationPair {
                            source: source_model.extract_layer_activations(source_layer, prompt)?,
                            target: target_model.extract_layer_activations(target_layer, prompt)?,
                        })
                    })
                    .collect()
            };
        let mut calibration = AlignmentCalibration {
            schema: "cerebro.cross_model.alignment_calibration/v1".into(),
            source_model: source_model.name().into(),
            target_model: target_model.name().into(),
            source_runtime_metadata_sha256: source_model.config().runtime_metadata_sha256.clone(),
            target_runtime_metadata_sha256: target_model.config().runtime_metadata_sha256.clone(),
            source_layer,
            target_layer,
            ridge_lambda: self.config.ridge_lambda,
            training_pairs: collect(training_prompts)?,
            validation_pairs: collect(validation_prompts)?,
            calibration_sha256: String::new(),
        };
        calibration.calibration_sha256 = calibration_digest(&calibration)?;
        calibration.validate()?;
        let fitted = self.fit(&calibration)?;
        if fitted.validation_residual > self.config.maximum_validation_residual {
            return Err(format!(
                "alignment_validation_residual_exceeded:{:.6}",
                fitted.validation_residual
            )
            .into());
        }
        Ok(calibration)
    }

    pub fn align(
        &self,
        source_vector: &Tensor,
        calibration: &AlignmentCalibration,
    ) -> Result<AlignmentResult, Box<dyn Error + Send + Sync>> {
        source_vector.validate()?;
        calibration.validate()?;
        if source_vector.layer_index != calibration.source_layer
            || source_vector.data.len() != calibration.training_pairs[0].source.data.len()
        {
            return Err("alignment_source_vector_binding_invalid".into());
        }
        let fitted = self.fit(calibration)?;
        if fitted.validation_residual > self.config.maximum_validation_residual {
            return Err("alignment_calibration_quality_gate_failed".into());
        }
        let target = predict(&fitted, source_vector)?;
        Ok(AlignmentResult {
            source_vector: source_vector.clone(),
            target_vector: target,
            alignment_score: (1.0 - fitted.validation_residual).clamp(0.0, 1.0),
            normalized_residual: fitted.validation_residual,
            calibration_sha256: fitted.calibration_sha256,
            method: AlignmentMethod::CalibratedLinearRidge,
        })
    }

    pub fn identity_verified(
        &self,
        vector: &Tensor,
        model: &dyn LLMModel,
    ) -> Result<AlignmentResult, String> {
        vector.validate()?;
        if vector.data.len() != model.embedding_dim() {
            return Err("identity_alignment_dimension_mismatch".into());
        }
        Ok(AlignmentResult {
            source_vector: vector.clone(),
            target_vector: vector.clone(),
            alignment_score: 1.0,
            normalized_residual: 0.0,
            calibration_sha256: model.config().runtime_metadata_sha256.clone(),
            method: AlignmentMethod::IdentityVerified,
        })
    }

    fn fit(
        &self,
        calibration: &AlignmentCalibration,
    ) -> Result<FittedRidgeMap, Box<dyn Error + Send + Sync>> {
        calibration.validate()?;
        let n = calibration.training_pairs.len();
        let target_dim = calibration.training_pairs[0].target.data.len();
        let source_training = calibration
            .training_pairs
            .iter()
            .map(|pair| pair.source.data.clone())
            .collect::<Vec<_>>();
        let target_training = calibration
            .training_pairs
            .iter()
            .map(|pair| pair.target.data.clone())
            .collect::<Vec<_>>();
        let mut gram = Matrix::zeros(n, n);
        for row in 0..n {
            for column in 0..n {
                let value = dot(&source_training[row], &source_training[column])
                    + if row == column {
                        calibration.ridge_lambda
                    } else {
                        0.0
                    };
                gram.set(row, column, value);
            }
        }
        let mut inverse = vec![vec![0.0; n]; n];
        for column in 0..n {
            let mut basis = vec![0.0; n];
            basis[column] = 1.0;
            let solution = solve(gram.clone(), basis)?;
            for row in 0..n {
                inverse[row][column] = solution[row];
            }
        }
        let mut coefficients = vec![vec![0.0; target_dim]; n];
        for row in 0..n {
            for target_column in 0..target_dim {
                coefficients[row][target_column] = (0..n)
                    .map(|index| inverse[row][index] * target_training[index][target_column])
                    .sum();
            }
        }
        let mut fitted = FittedRidgeMap {
            source_layer: calibration.source_layer,
            target_layer: calibration.target_layer,
            source_training,
            coefficients,
            calibration_sha256: calibration.calibration_sha256.clone(),
            validation_residual: 0.0,
        };
        fitted.validation_residual = validation_residual(&fitted, &calibration.validation_pairs)?;
        Ok(fitted)
    }
}

fn predict(map: &FittedRidgeMap, source: &Tensor) -> Result<Tensor, String> {
    let source_dim = map
        .source_training
        .first()
        .map(Vec::len)
        .ok_or("alignment_map_empty")?;
    let target_dim = map
        .coefficients
        .first()
        .map(Vec::len)
        .ok_or("alignment_coefficients_empty")?;
    if source.data.len() != source_dim || source.layer_index != map.source_layer {
        return Err("alignment_query_invalid".into());
    }
    let kernel = map
        .source_training
        .iter()
        .map(|row| dot(&source.data, row))
        .collect::<Vec<_>>();
    let mut target = vec![0.0; target_dim];
    for (column, value) in target.iter_mut().enumerate() {
        *value = (0..kernel.len())
            .map(|row| kernel[row] * map.coefficients[row][column])
            .sum();
    }
    let result = Tensor::new(
        target,
        vec![target_dim],
        Device::Cpu,
        DType::F64,
        map.target_layer,
    );
    result.validate()?;
    Ok(result)
}

fn validation_residual(map: &FittedRidgeMap, pairs: &[ActivationPair]) -> Result<f64, String> {
    let mut error_sq = 0.0;
    let mut target_sq = 0.0;
    for pair in pairs {
        let predicted = predict(map, &pair.source)?;
        if predicted.data.len() != pair.target.data.len() {
            return Err("alignment_validation_dimension_mismatch".into());
        }
        for (prediction, target) in predicted.data.iter().zip(&pair.target.data) {
            error_sq += (prediction - target).powi(2);
            target_sq += target.powi(2);
        }
    }
    if target_sq <= f64::EPSILON {
        return Err("alignment_validation_target_degenerate".into());
    }
    Ok((error_sq / target_sq).sqrt())
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn calibration_digest(calibration: &AlignmentCalibration) -> Result<String, String> {
    let mut unsigned = calibration.clone();
    unsigned.calibration_sha256.clear();
    serde_json::to_vec(&unsigned)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("alignment_calibration_serialize:{error}"))
}
impl Default for CrossModelAligner {
    fn default() -> Self {
        Self::new(CrossModelAlignerConfig::default()).expect("static aligner config")
    }
}
