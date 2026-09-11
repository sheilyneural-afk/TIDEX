//! Cross-model candidate readiness. Production authorization remains in the core.

pub mod domain_fitness;
pub mod evidence_validator;
pub mod promoter;
pub mod promotion_gates;

pub use domain_fitness::{DomainFitness, DomainFitnessConfig, DomainFitnessResult};
pub use evidence_validator::{
    EvidenceItem, EvidenceType, EvidenceValidator, EvidenceValidatorConfig, ValidationResult,
};
pub use promoter::{
    Promoter, PromoterConfig, PromotionResult, PromotionStage, PromotionStatistics,
};
pub use promotion_gates::{GateType, PromotionGateConfig, PromotionGateResult, PromotionGates};
