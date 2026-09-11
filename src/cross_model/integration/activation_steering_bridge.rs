//! Thin adapter to the canonical activation-steering materializer.

use crate::foundation::error::BrainResult;
use crate::materialization::activation_steering_materializer::{
    materialize_replayed_activation_steering_shadow, ActivationSteeringLayout,
    ActivationSteeringPolicy, ShadowActivationSteeringCandidate,
};
use crate::materialization::universal_capability_compiler::{
    UniversalCapabilityPlanningRequest, UniversalCapabilityShadowPlanReceipt,
};
use crate::receiver::receiver_layout::ReceiverMaterializationLayout;

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
