//! Thin adapter to the canonical activation-steering materializer.

use crate::activation_steering_materializer::{
    materialize_replayed_activation_steering_shadow, ActivationSteeringLayout,
    ActivationSteeringPolicy, ShadowActivationSteeringCandidate,
};
use crate::error::BrainResult;
use crate::receiver_layout::ReceiverMaterializationLayout;
use crate::universal_capability_compiler::{
    UniversalCapabilityPlanningRequest, UniversalCapabilityShadowPlanReceipt,
};

#[derive(Debug, Clone, Default)]
pub struct ActivationSteeringBridgeConfig;

#[derive(Debug, Clone, Default)]
pub struct ActivationSteeringBridge;

impl ActivationSteeringBridge {
    pub fn new(_config: ActivationSteeringBridgeConfig) -> Self {
        Self
    }

    pub fn materialize_steering(
        &self,
        request: &UniversalCapabilityPlanningRequest,
        receipt: &UniversalCapabilityShadowPlanReceipt,
        receiver_layout: &ReceiverMaterializationLayout,
        steering_layout: &ActivationSteeringLayout,
        policy: &ActivationSteeringPolicy,
    ) -> BrainResult<ShadowActivationSteeringCandidate> {
        materialize_replayed_activation_steering_shadow(
            request,
            receipt,
            receiver_layout,
            steering_layout,
            policy,
        )
    }
}
