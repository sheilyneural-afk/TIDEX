//! Co-evolution module
//!
//! This module contains components for implementing co-evolution
//! between models and capabilities in the cross-model system.

pub mod bidirectional_loop;
pub mod consensus_builder;

pub use bidirectional_loop::{
    AppliedInterventionEvidence, BidirectionalLoop, BidirectionalLoopConfig, CoEvolutionProgress,
    CoEvolutionStep,
};
pub use consensus_builder::{
    ConsensusBuilder, ConsensusBuilderConfig, ConsensusProposal, ConsensusState,
    ConsensusStatistics, ProposalType, Vote,
};
