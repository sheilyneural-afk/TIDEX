//! Evidence-governed cross-model orchestrator.
//!
//! The engine separates four phases that earlier code conflated:
//! behavioral discovery, internal evidence acquisition, calibrated
//! transformation, and physical intervention. No phase can manufacture the
//! evidence required by a later phase.

use crate::cross_model::discovery::{
    evaluate_model, BehavioralBenchmark, CapabilityGap, GapDetector, ModelEvaluation, Prioritizer,
    PriorityScore, ProposalGenerator, TransferProposal,
};
use crate::cross_model::extraction::{
    AlignmentCalibration, CrossModelAligner, ExtractionLevel, ExtractionResult,
    HierarchicalSteeringExtractor,
};
use crate::cross_model::models::{
    ActivationInterventionReceipt, AlignmentResult, CapabilityMetadata, LLMModel, ModelAccess,
    SteeringVector,
};
use crate::cross_model::promotion::{EvidenceItem, Promoter, PromotionResult, PromotionStatistics};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlasticityEngineConfig {
    pub maximum_models: usize,
    pub maximum_stored_capabilities: usize,
}

impl Default for PlasticityEngineConfig {
    fn default() -> Self {
        Self {
            maximum_models: 64,
            maximum_stored_capabilities: 100_000,
        }
    }
}

impl PlasticityEngineConfig {
    fn validate(&self) -> Result<(), String> {
        if self.maximum_models < 2
            || self.maximum_models > 4096
            || self.maximum_stored_capabilities == 0
        {
            return Err("plasticity_engine_config_invalid".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlasticityEngineState {
    pub models_registered: usize,
    pub behavioral_evaluations_completed: usize,
    pub capability_gaps_discovered: usize,
    pub internal_capabilities_acquired: usize,
    pub calibrated_alignments_completed: usize,
    pub activation_interventions_completed: usize,
    pub last_activity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryCycleReport {
    pub schema: String,
    pub benchmark_id: String,
    pub evaluations: Vec<ModelEvaluation>,
    pub gaps: Vec<CapabilityGap>,
    pub priorities: Vec<PriorityScore>,
    pub proposals: Vec<TransferProposal>,
}

pub struct PlasticityEngine {
    config: PlasticityEngineConfig,
    state: PlasticityEngineState,
    gap_detector: GapDetector,
    prioritizer: Prioritizer,
    proposal_generator: ProposalGenerator,
    hierarchical_extractor: HierarchicalSteeringExtractor,
    cross_model_aligner: CrossModelAligner,
    promoter: Promoter,
    registered_models: Vec<Box<dyn LLMModel>>,
    discovered_capabilities: HashMap<String, CapabilityMetadata>,
    steering_vectors: HashMap<String, SteeringVector>,
    alignment_calibrations: HashMap<String, AlignmentCalibration>,
    last_evaluations: BTreeMap<(String, String), ModelEvaluation>,
}

impl PlasticityEngine {
    pub fn new(config: PlasticityEngineConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            config,
            state: PlasticityEngineState {
                models_registered: 0,
                behavioral_evaluations_completed: 0,
                capability_gaps_discovered: 0,
                internal_capabilities_acquired: 0,
                calibrated_alignments_completed: 0,
                activation_interventions_completed: 0,
                last_activity: chrono::Utc::now().to_rfc3339(),
            },
            gap_detector: GapDetector::default(),
            prioritizer: Prioritizer::default(),
            proposal_generator: ProposalGenerator::default(),
            hierarchical_extractor: HierarchicalSteeringExtractor::default(),
            cross_model_aligner: CrossModelAligner::default(),
            promoter: Promoter::default(),
            registered_models: Vec::new(),
            discovered_capabilities: HashMap::new(),
            steering_vectors: HashMap::new(),
            alignment_calibrations: HashMap::new(),
            last_evaluations: BTreeMap::new(),
        })
    }

    pub fn register_model(&mut self, model: Box<dyn LLMModel>) -> Result<(), String> {
        model.config().validate()?;
        if self.registered_models.len() >= self.config.maximum_models {
            return Err("registered_model_limit".into());
        }
        if self
            .registered_models
            .iter()
            .any(|existing| existing.name() == model.name())
        {
            return Err("registered_model_duplicate".into());
        }
        self.registered_models.push(model);
        self.state.models_registered = self.registered_models.len();
        self.touch();
        Ok(())
    }

    pub fn run_discovery_pipeline(
        &mut self,
        benchmark: &BehavioralBenchmark,
    ) -> Result<DiscoveryCycleReport, Box<dyn Error + Send + Sync>> {
        benchmark.validate()?;
        if self.registered_models.len() < 2 {
            return Err("discovery_requires_two_models".into());
        }
        let mut evaluations = Vec::with_capacity(self.registered_models.len());
        for model in &self.registered_models {
            let evaluation = evaluate_model(model.as_ref(), benchmark)?;
            self.last_evaluations.insert(
                (benchmark.benchmark_id.clone(), model.name().to_string()),
                evaluation.clone(),
            );
            evaluations.push(evaluation);
        }
        let mut gaps = Vec::new();
        for first in 0..evaluations.len() {
            for second in first + 1..evaluations.len() {
                if let Some(gap) = self.gap_detector.detect_from_evaluations(
                    &evaluations[first],
                    &evaluations[second],
                    benchmark,
                )? {
                    let metadata = gap.capability_metadata()?;
                    self.insert_capability(metadata)?;
                    gaps.push(gap);
                }
            }
        }
        // Pairwise gaps of the same benchmark can repeat the capability name.
        // Prioritize each pair independently, then use a unique pair key below.
        let mut priorities = Vec::new();
        let mut proposals = Vec::new();
        for gap in &gaps {
            let score = self.prioritizer.prioritize(std::slice::from_ref(gap))?;
            if let Some(priority) = score.into_iter().next() {
                let generated = self.proposal_generator.generate_proposals(
                    std::slice::from_ref(gap),
                    std::slice::from_ref(&priority),
                )?;
                priorities.push(priority);
                proposals.extend(generated);
            }
        }
        self.state.behavioral_evaluations_completed = self
            .state
            .behavioral_evaluations_completed
            .checked_add(evaluations.len())
            .ok_or("evaluation_counter_overflow")?;
        self.state.capability_gaps_discovered = self
            .state
            .capability_gaps_discovered
            .checked_add(gaps.len())
            .ok_or("gap_counter_overflow")?;
        self.touch();
        Ok(DiscoveryCycleReport {
            schema: "cerebro.cross_model.discovery_cycle/v1".into(),
            benchmark_id: benchmark.benchmark_id.clone(),
            evaluations,
            gaps,
            priorities,
            proposals,
        })
    }

    pub fn extract_internal_capability(
        &mut self,
        model_name: &str,
        capability_name: &str,
        domain: &str,
        level: ExtractionLevel,
        positive_examples: &[String],
        negative_examples: &[String],
    ) -> Result<ExtractionResult, Box<dyn Error + Send + Sync>> {
        let model = self.model(model_name)?;
        let result = self.hierarchical_extractor.extract(
            model,
            capability_name,
            domain,
            level,
            positive_examples,
            negative_examples,
        )?;
        self.insert_capability(result.steering_vector.metadata.clone())?;
        self.steering_vectors
            .insert(capability_name.into(), result.steering_vector.clone());
        self.state.internal_capabilities_acquired = self
            .state
            .internal_capabilities_acquired
            .checked_add(1)
            .ok_or("internal_capability_counter_overflow")?;
        self.touch();
        Ok(result)
    }

    pub fn calibrate_alignment(
        &mut self,
        source_model: &str,
        target_model: &str,
        source_layer: usize,
        target_layer: usize,
        training_prompts: &[String],
        validation_prompts: &[String],
    ) -> Result<AlignmentCalibration, Box<dyn Error + Send + Sync>> {
        let source = self.model(source_model)?;
        let target = self.model(target_model)?;
        let calibration = self.cross_model_aligner.calibrate_from_prompts(
            source,
            target,
            source_layer,
            target_layer,
            training_prompts,
            validation_prompts,
        )?;
        self.alignment_calibrations
            .insert(calibration.calibration_sha256.clone(), calibration.clone());
        self.state.calibrated_alignments_completed = self
            .state
            .calibrated_alignments_completed
            .checked_add(1)
            .ok_or("alignment_counter_overflow")?;
        self.touch();
        Ok(calibration)
    }

    pub fn align_capability(
        &self,
        capability_name: &str,
        calibration_sha256: &str,
    ) -> Result<AlignmentResult, Box<dyn Error + Send + Sync>> {
        let steering = self
            .steering_vectors
            .get(capability_name)
            .ok_or("steering_vector_missing")?;
        let calibration = self
            .alignment_calibrations
            .get(calibration_sha256)
            .ok_or("alignment_calibration_missing")?;
        if steering.metadata.source_model != calibration.source_model {
            return Err("steering_alignment_source_model_mismatch".into());
        }
        self.cross_model_aligner
            .align(&steering.vector, calibration)
    }

    pub fn apply_aligned_activation_intervention(
        &mut self,
        capability_name: &str,
        target_model_name: &str,
        alignment: &AlignmentResult,
        strength: f64,
    ) -> Result<ActivationInterventionReceipt, Box<dyn Error + Send + Sync>> {
        let steering = self
            .steering_vectors
            .get(capability_name)
            .ok_or("steering_vector_missing")?;
        if alignment.source_vector != steering.vector || !strength.is_finite() || strength <= 0.0 {
            return Err("activation_intervention_request_invalid".into());
        }
        let target = self.model(target_model_name)?;
        // Ensure the provided alignment corresponds to a known calibration and
        // that the calibration was produced for this exact target model.
        let calibration = self
            .alignment_calibrations
            .get(&alignment.calibration_sha256)
            .ok_or("alignment_calibration_unknown")?;
        if calibration.target_model != target_model_name
            || calibration.target_runtime_metadata_sha256 != target.config().runtime_metadata_sha256
        {
            return Err("alignment_calibration_target_mismatch".into());
        }

        if !target.supports(ModelAccess::ActivationIntervention) {
            return Err(
                format!("target_intervention_backend_unavailable:{target_model_name}").into(),
            );
        }
        let receipt = target.apply_steering(
            alignment.target_vector.layer_index,
            &alignment.target_vector,
            strength,
        )?;
        receipt.validate()?;
        self.state.activation_interventions_completed = self
            .state
            .activation_interventions_completed
            .checked_add(1)
            .ok_or("intervention_counter_overflow")?;
        self.touch();
        Ok(receipt)
    }

    pub fn clear_model_activation_interventions(
        &mut self,
        target_model_name: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let target = self.model(target_model_name)?;
        if !target.supports(ModelAccess::ActivationIntervention) {
            return Err(
                format!("target_intervention_backend_unavailable:{target_model_name}").into(),
            );
        }
        target.clear_activation_interventions()?;
        self.touch();
        Ok(())
    }

    pub fn inspect_internal_module(
        &self,
        model_name: &str,
        request: &crate::cross_model::models::DeepInstrumentationRequest,
    ) -> Result<crate::cross_model::models::DeepInstrumentationEvidence, Box<dyn Error + Send + Sync>>
    {
        let model = self.model(model_name)?;
        if !model.supports(ModelAccess::DeepInstrumentation) {
            return Err(format!("deep_instrumentation_backend_unavailable:{model_name}").into());
        }
        model.deep_instrumentation(request)
    }

    pub fn analyze_sparse_features(
        &self,
        model_name: &str,
        request: &crate::cross_model::models::SparseAutoencoderRequest,
    ) -> Result<crate::cross_model::models::SparseAutoencoderEvidence, Box<dyn Error + Send + Sync>>
    {
        let model = self.model(model_name)?;
        if !model.supports(ModelAccess::SparseAutoencoderAnalysis) {
            return Err(format!("sparse_autoencoder_backend_unavailable:{model_name}").into());
        }
        model.sparse_autoencoder_analysis(request)
    }

    pub fn add_promotion_evidence(
        &mut self,
        capability_name: &str,
        evidence: EvidenceItem,
    ) -> Result<(), String> {
        self.promoter.add_evidence(capability_name, evidence)
    }

    pub fn evaluate_candidate_readiness(
        &mut self,
        capability_name: &str,
    ) -> Result<PromotionResult, String> {
        let capability = self
            .discovered_capabilities
            .get(capability_name)
            .cloned()
            .ok_or("capability_missing")?;
        let result = self.promoter.evaluate(&capability, None)?;
        self.touch();
        Ok(result)
    }

    pub fn last_evaluation(&self, benchmark_id: &str, model: &str) -> Option<&ModelEvaluation> {
        self.last_evaluations
            .get(&(benchmark_id.into(), model.into()))
    }

    pub fn get_state(&self) -> &PlasticityEngineState {
        &self.state
    }

    pub fn get_models(&self) -> Vec<&str> {
        self.registered_models
            .iter()
            .map(|model| model.name())
            .collect()
    }

    pub fn get_capabilities(&self) -> Vec<&CapabilityMetadata> {
        let mut values = self.discovered_capabilities.values().collect::<Vec<_>>();
        values.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.source_model.cmp(&b.source_model))
        });
        values
    }

    pub fn get_promotion_statistics(&self) -> PromotionStatistics {
        self.promoter.get_statistics()
    }

    pub fn reset(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        for model in &self.registered_models {
            if model.supports(ModelAccess::ActivationIntervention) {
                model.clear_activation_interventions()?;
            }
        }
        self.discovered_capabilities.clear();
        self.steering_vectors.clear();
        self.alignment_calibrations.clear();
        self.last_evaluations.clear();
        self.promoter.clear_history();
        self.state = PlasticityEngineState {
            models_registered: self.registered_models.len(),
            behavioral_evaluations_completed: 0,
            capability_gaps_discovered: 0,
            internal_capabilities_acquired: 0,
            calibrated_alignments_completed: 0,
            activation_interventions_completed: 0,
            last_activity: chrono::Utc::now().to_rfc3339(),
        };
        Ok(())
    }

    fn model(&self, name: &str) -> Result<&dyn LLMModel, Box<dyn Error + Send + Sync>> {
        self.registered_models
            .iter()
            .find(|model| model.name() == name)
            .map(|model| model.as_ref())
            .ok_or_else(|| format!("registered_model_not_found:{name}").into())
    }

    fn insert_capability(&mut self, metadata: CapabilityMetadata) -> Result<(), String> {
        metadata.validate()?;
        if self.discovered_capabilities.len() >= self.config.maximum_stored_capabilities
            && !self.discovered_capabilities.contains_key(&metadata.name)
        {
            return Err("stored_capability_limit".into());
        }
        // Same semantic name may be observed from different pairs. The strongest
        // confidence wins only when the evidence kind is identical; an internal
        // activation artifact can replace a behavioral-only record, never vice versa.
        match self.discovered_capabilities.get(&metadata.name) {
            None => { self.discovered_capabilities.insert(metadata.name.clone(), metadata); }
            Some(existing) if existing.evidence_kind == metadata.evidence_kind && metadata.confidence > existing.confidence => {
                self.discovered_capabilities.insert(metadata.name.clone(), metadata);
            }
            Some(existing) if existing.evidence_kind != metadata.evidence_kind
                && metadata.evidence_kind != crate::cross_model::models::CapabilityEvidenceKind::BehavioralVerified => {
                self.discovered_capabilities.insert(metadata.name.clone(), metadata);
            }
            _ => {}
        }
        Ok(())
    }

    fn touch(&mut self) {
        self.state.last_activity = chrono::Utc::now().to_rfc3339();
    }
}

impl Default for PlasticityEngine {
    fn default() -> Self {
        Self::new(PlasticityEngineConfig::default()).expect("static engine config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_creation_and_reset_are_fail_closed_empty() {
        let mut engine = PlasticityEngine::default();
        assert_eq!(engine.get_state().models_registered, 0);
        engine.reset().unwrap();
        assert_eq!(engine.get_state().activation_interventions_completed, 0);
    }
}
