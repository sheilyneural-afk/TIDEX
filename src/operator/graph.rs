//! Production Operator graph.
//!
//! Composes `executor_catalog` and `recipe_catalog` into a closed topology of
//! artifacts, reachability and authority. This module is part of the runtime,
//! not a test harness. Quality gates invoke it; they do not replace it.

use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::BrainResult;
use crate::operator::artifact::{ArtifactKind, ArtifactRole};
use crate::operator::control_plane::recipe_catalog;
use crate::operator::executor_registry::{
    direct_runner_operation_bindings, executor_catalog, ExecutorDescriptor, ExecutorState,
    ExecutorSurface,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const GRAPH_SCHEMA: &str = "cerebro.tidex.operator_graph/v1";
const GRAPH_DOMAIN: &[u8] = b"CEREBRO:TIDEX:OPERATOR-GRAPH:v1\0";
const DIRECT_RUNNER_RECIPE: &str = "operator.direct_runner";
const PRODUCTION_AUTHORITY_EXECUTOR: &str = "adapter.bank";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphFindingKind {
    RecipeWithoutExecutor,
    ExecutorRecipeUnknown,
    MultiplexedRecipeUnresolved,
    MissingProducer,
    DeadEnd,
    UnreachableOperational,
    InvalidProductionAuthority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GraphFinding {
    pub kind: GraphFindingKind,
    pub subject: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorGraphReceipt {
    pub schema: String,
    pub executor_count: usize,
    pub recipe_count: usize,
    pub artifact_count: usize,
    pub multiplex_bindings: usize,
    pub findings: Vec<GraphFinding>,
    pub passed: bool,
    pub graph_sha256: Sha256Digest,
}

fn has_entry_surface(executor: &ExecutorDescriptor) -> bool {
    executor.surfaces.iter().any(|surface| {
        matches!(
            surface,
            ExecutorSurface::TidexCli
                | ExecutorSurface::TidexOperator
                | ExecutorSurface::Binary
                | ExecutorSurface::CoreEngine
                | ExecutorSurface::CrossModelDaemon
        )
    })
}

fn operational_for_reachability(executor: &ExecutorDescriptor) -> bool {
    matches!(
        executor.state,
        ExecutorState::Operational | ExecutorState::OperationalNeedsWorkflow
    ) || executor.actionable_now
}

pub fn compute_operator_graph() -> BrainResult<OperatorGraphReceipt> {
    let executors = executor_catalog()?;
    let recipes = recipe_catalog();
    let recipe_ids = recipes
        .iter()
        .map(|recipe| recipe.id.clone())
        .collect::<BTreeSet<_>>();
    let mut findings = Vec::new();

    let mut producers: BTreeMap<ArtifactKind, Vec<String>> = BTreeMap::new();
    let mut consumers: BTreeMap<ArtifactKind, Vec<String>> = BTreeMap::new();
    let mut artifacts = BTreeSet::new();
    for executor in &executors {
        for artifact in &executor.produces {
            artifacts.insert(*artifact);
            producers
                .entry(*artifact)
                .or_default()
                .push(executor.executor_id.clone());
        }
        for artifact in &executor.requires {
            artifacts.insert(*artifact);
            consumers
                .entry(*artifact)
                .or_default()
                .push(executor.executor_id.clone());
        }
        if let Some(recipe_id) = executor.operator_recipe_id.as_deref() {
            if !recipe_ids.contains(recipe_id) {
                findings.push(GraphFinding {
                    kind: GraphFindingKind::ExecutorRecipeUnknown,
                    subject: executor.executor_id.clone(),
                    detail: format!("operator_recipe_id {recipe_id} is not in recipe_catalog"),
                });
            }
        }
        if operational_for_reachability(executor) && !has_entry_surface(executor) {
            findings.push(GraphFinding {
                kind: GraphFindingKind::UnreachableOperational,
                subject: executor.executor_id.clone(),
                detail: "operational or actionable executor has no CLI, Operator, Binary, CoreEngine or daemon surface".into(),
            });
        }
        if executor.production_authority && executor.executor_id != PRODUCTION_AUTHORITY_EXECUTOR {
            findings.push(GraphFinding {
                kind: GraphFindingKind::InvalidProductionAuthority,
                subject: executor.executor_id.clone(),
                detail: format!(
                    "production_authority is reserved for {PRODUCTION_AUTHORITY_EXECUTOR}"
                ),
            });
        }
        if executor.executor_id == PRODUCTION_AUTHORITY_EXECUTOR && !executor.production_authority {
            findings.push(GraphFinding {
                kind: GraphFindingKind::InvalidProductionAuthority,
                subject: executor.executor_id.clone(),
                detail: "unique production authority is missing production_authority".into(),
            });
        }
    }

    for recipe in &recipes {
        let bound = executors
            .iter()
            .filter(|entry| entry.operator_recipe_id.as_deref() == Some(recipe.id.as_str()))
            .map(|entry| entry.executor_id.as_str())
            .collect::<Vec<_>>();
        if bound.is_empty() {
            findings.push(GraphFinding {
                kind: GraphFindingKind::RecipeWithoutExecutor,
                subject: recipe.id.clone(),
                detail: "recipe has no executor binding".into(),
            });
            continue;
        }
        if bound.len() > 1 && recipe.id != DIRECT_RUNNER_RECIPE {
            findings.push(GraphFinding {
                kind: GraphFindingKind::MultiplexedRecipeUnresolved,
                subject: recipe.id.clone(),
                detail: format!("recipe binds {} executors without an operation map", bound.len()),
            });
        }
    }

    let bound_direct: BTreeSet<&str> = executors
        .iter()
        .filter(|entry| entry.operator_recipe_id.as_deref() == Some(DIRECT_RUNNER_RECIPE))
        .map(|entry| entry.executor_id.as_str())
        .collect();
    let mapped_direct: BTreeSet<&str> = direct_runner_operation_bindings()
        .iter()
        .map(|(_, executor_id)| *executor_id)
        .collect();
    for executor_id in &bound_direct {
        if !mapped_direct.contains(executor_id) {
            findings.push(GraphFinding {
                kind: GraphFindingKind::MultiplexedRecipeUnresolved,
                subject: (*executor_id).into(),
                detail: format!("{DIRECT_RUNNER_RECIPE} executor has no direct-operation binding"),
            });
        }
    }

    for executor in &executors {
        if !operational_for_reachability(executor) {
            continue;
        }
        for artifact in &executor.requires {
            if artifact.role() == ArtifactRole::ExternalInput {
                continue;
            }
            if producers
                .get(artifact)
                .into_iter()
                .flatten()
                .next()
                .is_none()
            {
                findings.push(GraphFinding {
                    kind: GraphFindingKind::MissingProducer,
                    subject: executor.executor_id.clone(),
                    detail: format!("requires {} with no producer", artifact.as_str()),
                });
            }
        }
        for artifact in &executor.produces {
            if artifact.role() != ArtifactRole::Intermediate {
                continue;
            }
            let consumed = consumers
                .get(artifact)
                .map(|ids| ids.iter().any(|id| id != &executor.executor_id))
                .unwrap_or(false);
            if !consumed {
                findings.push(GraphFinding {
                    kind: GraphFindingKind::DeadEnd,
                    subject: executor.executor_id.clone(),
                    detail: format!("produces intermediate {} with no consumer", artifact.as_str()),
                });
            }
        }
    }

    findings.sort_by(|left, right| {
        left.kind
            .to_string()
            .cmp(right.kind.to_string())
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.detail.cmp(&right.detail))
    });
    let passed = findings.is_empty();
    let mut receipt = OperatorGraphReceipt {
        schema: GRAPH_SCHEMA.into(),
        executor_count: executors.len(),
        recipe_count: recipes.len(),
        artifact_count: artifacts.len(),
        multiplex_bindings: direct_runner_operation_bindings().len(),
        findings: findings.clone(),
        passed,
        graph_sha256: Sha256Digest::zero(),
    };
    receipt.graph_sha256 =
        Sha256Digest::digest_domain(GRAPH_DOMAIN, &serde_json::to_vec(&receipt)?);
    Ok(receipt)
}

impl GraphFindingKind {
    fn to_string(self) -> &'static str {
        match self {
            Self::RecipeWithoutExecutor => "recipe_without_executor",
            Self::ExecutorRecipeUnknown => "executor_recipe_unknown",
            Self::MultiplexedRecipeUnresolved => "multiplexed_recipe_unresolved",
            Self::MissingProducer => "missing_producer",
            Self::DeadEnd => "dead_end",
            Self::UnreachableOperational => "unreachable_operational",
            Self::InvalidProductionAuthority => "invalid_production_authority",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::artifact::ArtifactKind;
    use crate::operator::executor_registry::executor_catalog;

    #[test]
    fn freeze_and_compile_share_frozen_receiver_compiler() {
        let catalog = executor_catalog().unwrap();
        let freeze = catalog
            .iter()
            .find(|entry| entry.executor_id == "receiver.freeze_compiler")
            .unwrap();
        let compile = catalog
            .iter()
            .find(|entry| entry.executor_id == "receiver.compile_universal")
            .unwrap();
        assert!(freeze
            .produces
            .contains(&ArtifactKind::FrozenReceiverCompiler));
        assert!(compile
            .requires
            .contains(&ArtifactKind::FrozenReceiverCompiler));
    }

    #[test]
    fn direct_runner_multiplex_is_bound_by_operation() {
        let catalog = executor_catalog().unwrap();
        let bound: BTreeSet<_> = catalog
            .iter()
            .filter(|entry| entry.operator_recipe_id.as_deref() == Some(DIRECT_RUNNER_RECIPE))
            .map(|entry| entry.executor_id.as_str())
            .collect();
        let mapped: BTreeSet<_> = direct_runner_operation_bindings()
            .iter()
            .map(|(_, id)| *id)
            .collect();
        assert!(bound.iter().all(|id| mapped.contains(id)));
        let receipt = compute_operator_graph().unwrap();
        assert!(!receipt.findings.iter().any(|finding| {
            finding.kind == GraphFindingKind::MultiplexedRecipeUnresolved
                && finding.subject.starts_with("cross_model.")
        }));
    }

    #[test]
    fn graph_is_closed_for_registered_production_surface() {
        let receipt = compute_operator_graph().unwrap();
        assert_eq!(receipt.schema, GRAPH_SCHEMA);
        assert!(receipt.executor_count >= 30);
        assert!(receipt.recipe_count >= 30);
        assert!(receipt.passed, "findings={:?}", receipt.findings);
        assert!(receipt.findings.is_empty());
        assert_ne!(receipt.graph_sha256, Sha256Digest::zero());
    }

    #[test]
    fn closed_vocabulary_rejects_retired_frozen_compiler_token() {
        assert!(ArtifactKind::parse("frozen_compiler").is_err());
        assert_eq!(
            ArtifactKind::parse("frozen_receiver_compiler").unwrap(),
            ArtifactKind::FrozenReceiverCompiler
        );
    }
}
