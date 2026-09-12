//! Co-evolution module
//!
//! BidirectionalLoop is the operational advisory co-evolution loop: it seals
//! causally filtered discovery history and emits the next tick that steers
//! operator plasticity controllers without production authority.

pub mod bidirectional_loop;
pub mod consensus_builder;

pub use bidirectional_loop::{
    intervention_causally_allowed_for_cycle, AppliedInterventionEvidence, BidirectionalLoop,
    BidirectionalLoopConfig, CoEvolutionDirective, CoEvolutionProgress, CoEvolutionStep,
};
pub use consensus_builder::{
    ConsensusBuilder, ConsensusBuilderConfig, ConsensusProposal, ConsensusState,
    ConsensusStatistics, ProposalType, Vote,
};
