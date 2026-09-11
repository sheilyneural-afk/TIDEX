//! Plasticity module
//!
//! This module contains components for implementing adaptive
//! plasticity in the cross-model system.

pub mod bcm_metaplasticity;
pub mod content_plasticity;
pub mod eligibility_traces;
pub mod elo_system;
pub mod neuromodulation;
pub mod pi_controller;
pub mod routing_plasticity;

pub use bcm_metaplasticity::{BCMConfig, BCMMetaplasticity, BCMState};
pub use content_plasticity::{ContentPlasticity, ContentPlasticityConfig, ContentPlasticityState};
pub use eligibility_traces::{EligibilityTrace, EligibilityTraceConfig, EligibilityTraces};
pub use elo_system::{ELOConfig, ELOState, ELOSystem};
pub use neuromodulation::{
    Neuromodulation, NeuromodulationConfig, NeuromodulationSignal, Neuromodulator,
};
pub use pi_controller::{PIController, PIControllerConfig, PIControllerState};
pub use routing_plasticity::{
    RoutingDecision, RoutingObservation, RoutingPlasticity, RoutingPlasticityConfig,
    RoutingStatistics,
};
