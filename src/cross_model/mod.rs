//! Evidence-governed cross-model runtime.
//!
//! The module separates behavioral evaluation, measured internal activations,
//! calibrated representation transport, activation intervention, physical weight
//! materialization, and production authorization. No phase fabricates evidence
//! for the next one, and production activation remains a core TIDE-X authority.

pub mod co_evolution;
pub mod discovery;
pub mod extraction;
pub mod integration;
pub mod models;
pub mod plasticity;
pub mod plasticity_engine;
pub mod promotion;

// Re-export commonly used types
pub use models::{
    ActivationInterventionReceipt, AlignmentMethod, AlignmentResult, ArchitectureFamily,
    CandleLlamaModel, CandleMistralModel, CapabilityMetadata, DType, Device, GenerationPolicy,
    HfTransformersModel, HfTransformersRuntimeConfig, LLMModel, LlamaModel, MistralModel,
    ModelAccess, ModelConfig, OllamaModel, QwenModel, SteeringVector, Tensor,
};

pub use discovery::{
    BatchProposal, CapabilityGap, Domain, DomainAnalyzer, DomainAnalyzerConfig, DomainComparison,
    DomainProfile, EmergenceType, EmergentCapability, EmergentDetector, EmergentDetectorConfig,
    GapDetector, GapDetectorConfig, Prioritizer, PrioritizerConfig, Priority, PriorityScore,
    ProposalGenerator, ProposalGeneratorConfig, ResourceRequirements, TransferProposal,
};

pub use extraction::{
    CounterfactualAnalyzer, CounterfactualAnalyzerConfig, CounterfactualResult,
    CounterfactualScenario, CounterfactualSummary, CrossModelAligner, CrossModelAlignerConfig,
    ExtractionLevel, ExtractionResult, HierarchicalComponent, HierarchicalExtractorConfig,
    HierarchicalSteeringExtractor, LoRAConfig, LoRASynthesisResult, LoRASynthesizer,
    LoRASynthesizerConfig, LoRAWeights, PerturbationType, QualityMetrics,
};

pub use promotion::{
    DomainFitness, DomainFitnessConfig, DomainFitnessResult, EvidenceItem, EvidenceType,
    EvidenceValidator, EvidenceValidatorConfig, GateType, Promoter, PromoterConfig,
    PromotionGateConfig, PromotionGateResult, PromotionGates, PromotionResult, PromotionStage,
    PromotionStatistics, ValidationResult,
};

pub use integration::{
    ActivationSteeringBridge, ActivationSteeringBridgeConfig, AdapterBankBridge,
    AdapterBankBridgeConfig, CapabilityDiscoveryBridge, CapabilityDiscoveryBridgeConfig,
    CausalCreditBridge, CausalCreditBridgeConfig, LedgerBridge, LedgerBridgeConfig,
    PythagorasBridge, PythagorasBridgeConfig, ShadowEvaluationBridge, ShadowEvaluationBridgeConfig,
    TemporalTrackingBridge, TemporalTrackingBridgeConfig, WeightTomographyBridge,
    WeightTomographyBridgeConfig,
};

pub use plasticity::{
    BCMConfig, BCMMetaplasticity, BCMState, ContentPlasticity, ContentPlasticityConfig,
    ContentPlasticityState, ELOConfig, ELOState, ELOSystem, EligibilityTrace,
    EligibilityTraceConfig, EligibilityTraces, Neuromodulation, NeuromodulationConfig,
    NeuromodulationSignal, Neuromodulator, PIController, PIControllerConfig, PIControllerState,
    RoutingDecision, RoutingPlasticity, RoutingPlasticityConfig, RoutingStatistics,
};

pub use co_evolution::{
    BidirectionalLoop, BidirectionalLoopConfig, CoEvolutionProgress, CoEvolutionStep,
    ConsensusBuilder, ConsensusBuilderConfig, ConsensusProposal, ConsensusState,
    ConsensusStatistics, ProposalType, Vote,
};
