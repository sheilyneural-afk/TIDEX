//! Capability extraction and calibrated cross-model transformation.

pub mod counterfactual_analyzer;
pub mod cross_model_aligner;
pub mod hierarchical_steering_extractor;
pub mod lora_synthesizer;

pub use crate::cross_model::models::AlignmentResult;
pub use counterfactual_analyzer::{
    CounterfactualAnalyzer, CounterfactualAnalyzerConfig, CounterfactualResult,
    CounterfactualScenario, CounterfactualSummary, LayerInterventionObservation, PerturbationType,
};
pub use cross_model_aligner::{
    ActivationPair, AlignmentCalibration, CrossModelAligner, CrossModelAlignerConfig,
};
pub use hierarchical_steering_extractor::{
    ExtractionLevel, ExtractionResult, HierarchicalComponent, HierarchicalExtractorConfig,
    HierarchicalSteeringExtractor, QualityMetrics,
};
pub use lora_synthesizer::{
    LoRAConfig, LoRASynthesisResult, LoRASynthesizer, LoRASynthesizerConfig, LoRAWeights,
};
