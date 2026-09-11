//! Bridge from CEREBRO3's canonical discovery report to cross-model metadata.

use crate::capability_discovery::{
    CapabilityDiscoveryDisposition, CapabilityDiscoveryReport, DiscoveredCapabilityEvidence,
};
use crate::cross_model::models::{
    sha256_hex, CapabilityEvidenceKind, CapabilityMetadata, LLMModel,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CapabilityDiscoveryBridgeConfig {
    pub maximum_cache_size: usize,
}

impl Default for CapabilityDiscoveryBridgeConfig {
    fn default() -> Self {
        Self {
            maximum_cache_size: 10_000,
        }
    }
}

pub struct CapabilityDiscoveryBridge {
    config: CapabilityDiscoveryBridgeConfig,
    discovery_cache: HashMap<String, CapabilityMetadata>,
}

impl CapabilityDiscoveryBridge {
    pub fn new(config: CapabilityDiscoveryBridgeConfig) -> Result<Self, String> {
        if config.maximum_cache_size == 0 {
            return Err("discovery_bridge_cache_invalid".into());
        }
        Ok(Self {
            config,
            discovery_cache: HashMap::new(),
        })
    }

    pub fn bridge_capability(
        &mut self,
        report: &CapabilityDiscoveryReport,
        evidence: &DiscoveredCapabilityEvidence,
        model: &dyn LLMModel,
        domain: &str,
    ) -> Result<CapabilityMetadata, String> {
        model.config().validate()?;
        if report.schema != "cerebro.tidex.capability_discovery_report/v1"
            || domain.trim().is_empty()
            || evidence.disposition != CapabilityDiscoveryDisposition::EvidenceReadyForCapabilityIr
            || !report
                .capabilities
                .iter()
                .any(|candidate| candidate == evidence)
        {
            return Err("discovery_bridge_evidence_invalid".into());
        }
        let consistency = ((evidence.minimum_consistency + 1.0) / 2.0).clamp(0.0, 1.0);
        let control = ((evidence.minimum_control_margin + 2.0) / 4.0).clamp(0.0, 1.0);
        let confidence = consistency.min(control);
        let evidence_sha256 = sha256_hex(
            &serde_json::to_vec(&(
                report.manifest_sha256.as_str(),
                model.config().runtime_metadata_sha256.as_str(),
                evidence,
                domain,
            ))
            .map_err(|error| format!("discovery_bridge_serialize:{error}"))?,
        );
        let metadata = CapabilityMetadata {
            name: evidence.probe_id.clone(),
            source_model: model.name().into(),
            source_layer: None,
            domain: domain.into(),
            confidence,
            evidence_kind: CapabilityEvidenceKind::BehavioralVerified,
            evidence_sha256,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        metadata.validate()?;
        if self.discovery_cache.len() >= self.config.maximum_cache_size
            && !self.discovery_cache.contains_key(&metadata.name)
        {
            return Err("discovery_bridge_cache_limit".into());
        }
        self.discovery_cache
            .insert(metadata.name.clone(), metadata.clone());
        Ok(metadata)
    }

    pub fn batch_bridge(
        &mut self,
        report: &CapabilityDiscoveryReport,
        model: &dyn LLMModel,
        domain: &str,
    ) -> Result<Vec<CapabilityMetadata>, String> {
        report
            .capabilities
            .iter()
            .filter(|evidence| {
                evidence.disposition == CapabilityDiscoveryDisposition::EvidenceReadyForCapabilityIr
            })
            .map(|evidence| self.bridge_capability(report, evidence, model, domain))
            .collect()
    }

    pub fn get_cached_capability(&self, name: &str) -> Option<&CapabilityMetadata> {
        self.discovery_cache.get(name)
    }

    pub fn get_all_cached(&self) -> Vec<&CapabilityMetadata> {
        let mut values = self.discovery_cache.values().collect::<Vec<_>>();
        values.sort_by(|a, b| a.name.cmp(&b.name));
        values
    }

    pub fn clear_cache(&mut self) {
        self.discovery_cache.clear();
    }
}
impl Default for CapabilityDiscoveryBridge {
    fn default() -> Self {
        Self::new(CapabilityDiscoveryBridgeConfig::default())
            .expect("static discovery bridge config")
    }
}
