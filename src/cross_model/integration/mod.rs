//! Integration bridges module
//!
//! This module contains bridges to existing TIDE-X systems
//! for seamless integration with the cross-model system.

pub mod activation_steering_bridge;
pub mod adapter_bank_bridge;
pub mod capability_discovery_bridge;
pub mod causal_credit_bridge;
pub mod ledger_bridge;
pub mod pythagoras_bridge;
pub mod shadow_evaluation_bridge;
pub mod temporal_tracking_bridge;
pub mod weight_tomography_bridge;

pub use activation_steering_bridge::{ActivationSteeringBridge, ActivationSteeringBridgeConfig};
pub use adapter_bank_bridge::{AdapterBankBridge, AdapterBankBridgeConfig};
pub use capability_discovery_bridge::{CapabilityDiscoveryBridge, CapabilityDiscoveryBridgeConfig};
pub use causal_credit_bridge::{CausalCreditBridge, CausalCreditBridgeConfig};
pub use ledger_bridge::{LedgerBridge, LedgerBridgeConfig};
pub use pythagoras_bridge::{PythagorasBridge, PythagorasBridgeConfig};
pub use shadow_evaluation_bridge::{ShadowEvaluationBridge, ShadowEvaluationBridgeConfig};
pub use temporal_tracking_bridge::{TemporalTrackingBridge, TemporalTrackingBridgeConfig};
pub use weight_tomography_bridge::{WeightTomographyBridge, WeightTomographyBridgeConfig};
