//! Canonical TIDE-X executor capability registry.
//!
//! This registry is descriptive, not authoritative by itself: it declares which
//! existing implementation owns each executable capability, which effects it may
//! produce, and how it is exposed. Execution still goes through the owning
//! module, KnowledgeEngine/Operator receipts, and the normal governance path.

use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::operator::artifact::ArtifactKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorState {
    Operational,
    OperationalNeedsWorkflow,
    ExperimentalCandidateOnly,
    ArchitectureOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorEffectClass {
    ReadOnlyEvidence,
    PlanningOnly,
    CandidateArtifact,
    RuntimeIntervention,
    PhysicalMaterialization,
    GovernanceReadiness,
    ProductionLifecycle,
    LifecycleOperation,
    AdvisorySignal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorAuthorityClass {
    None,
    EvidenceProducer,
    EvidenceReducer,
    Planner,
    Executor,
    ReadinessGate,
    LifecycleAuthority,
    AdvisoryOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorSurface {
    CoreEngine,
    TidexCli,
    TidexOperator,
    Binary,
    InternalLibrary,
    CrossModelDaemon,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorImplementationStatus {
    Implemented,
    Partial,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorRuntimeStatus {
    RunnableNow,
    RunnableWithTypedInput,
    AdvisoryFromEvidence,
    CandidateOnly,
    RequiresWorkflow,
    RequiresBackendOrArtifact,
    BlockedByPolicy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorWorkflowStatus {
    OperatorReady,
    CliReady,
    EngineReady,
    InternalReady,
    NeedsOperatorWorkflow,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorEvidenceStatus {
    EmitsReceipt,
    EmitsEvidence,
    ConsumesEvidence,
    AdvisoryOnly,
    RequiresExternalEvidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorMaturity {
    ProductionLifecycle,
    Operational,
    OperationalCandidate,
    OperationalAdvisory,
    Experimental,
    NeedsWorkflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutorDescriptor {
    pub schema: String,
    pub executor_id: String,
    pub title: String,
    pub module_path: String,
    pub owner: String,
    /// Legacy compatibility field. In v2 semantics this means the executor has
    /// a registered implementation contract, not that it authorizes production.
    pub state: ExecutorState,
    pub implementation_status: ExecutorImplementationStatus,
    pub runtime_status: ExecutorRuntimeStatus,
    pub workflow_status: ExecutorWorkflowStatus,
    pub evidence_status: ExecutorEvidenceStatus,
    pub maturity: ExecutorMaturity,
    pub production_authority: bool,
    pub actionable_now: bool,
    pub authority: ExecutorAuthorityClass,
    pub effect_class: ExecutorEffectClass,
    pub surfaces: Vec<ExecutorSurface>,
    pub accepted_needs: Vec<String>,
    pub requires: Vec<ArtifactKind>,
    pub produces: Vec<ArtifactKind>,
    pub operator_recipe_id: Option<String>,
    pub notes: String,
    pub descriptor_sha256: Sha256Digest,
}

pub struct ExecutorDescriptorDraft<'a> {
    pub executor_id: &'a str,
    pub title: &'a str,
    pub module_path: &'a str,
    pub owner: &'a str,
    pub state: ExecutorState,
    pub authority: ExecutorAuthorityClass,
    pub effect_class: ExecutorEffectClass,
    pub surfaces: &'a [ExecutorSurface],
    pub accepted_needs: &'a [&'a str],
    pub requires: &'a [&'a str],
    pub produces: &'a [&'a str],
    pub operator_recipe_id: Option<&'a str>,
    pub notes: &'a str,
}

fn parse_artifact_list(items: &[&str]) -> BrainResult<Vec<ArtifactKind>> {
    items.iter().copied().map(ArtifactKind::parse).collect()
}

fn derive_workflow_status(
    legacy_state: ExecutorState,
    surfaces: &[ExecutorSurface],
    operator_recipe_id: Option<&str>,
) -> ExecutorWorkflowStatus {
    if operator_recipe_id.is_some() || surfaces.contains(&ExecutorSurface::TidexOperator) {
        ExecutorWorkflowStatus::OperatorReady
    } else if surfaces.contains(&ExecutorSurface::TidexCli)
        || surfaces.contains(&ExecutorSurface::Binary)
    {
        ExecutorWorkflowStatus::CliReady
    } else if surfaces.contains(&ExecutorSurface::CoreEngine) {
        ExecutorWorkflowStatus::EngineReady
    } else if legacy_state == ExecutorState::ArchitectureOnly {
        ExecutorWorkflowStatus::InternalReady
    } else {
        ExecutorWorkflowStatus::NeedsOperatorWorkflow
    }
}

fn derive_runtime_status(
    legacy_state: ExecutorState,
    authority: ExecutorAuthorityClass,
    effect: ExecutorEffectClass,
    workflow: ExecutorWorkflowStatus,
) -> ExecutorRuntimeStatus {
    match (legacy_state, authority, effect, workflow) {
        (_, ExecutorAuthorityClass::AdvisoryOnly, ExecutorEffectClass::AdvisorySignal, _) => {
            ExecutorRuntimeStatus::AdvisoryFromEvidence
        }
        (_, _, ExecutorEffectClass::ProductionLifecycle, _) => {
            ExecutorRuntimeStatus::RunnableWithTypedInput
        }
        (ExecutorState::ExperimentalCandidateOnly, _, _, _) => ExecutorRuntimeStatus::CandidateOnly,
        (ExecutorState::ArchitectureOnly, _, _, _) => ExecutorRuntimeStatus::AdvisoryFromEvidence,
        (ExecutorState::OperationalNeedsWorkflow, _, _, ExecutorWorkflowStatus::OperatorReady)
        | (ExecutorState::OperationalNeedsWorkflow, _, _, ExecutorWorkflowStatus::CliReady)
        | (ExecutorState::OperationalNeedsWorkflow, _, _, ExecutorWorkflowStatus::EngineReady) => {
            ExecutorRuntimeStatus::RunnableWithTypedInput
        }
        (ExecutorState::OperationalNeedsWorkflow, _, _, _) => {
            ExecutorRuntimeStatus::RequiresWorkflow
        }
        (ExecutorState::Operational, _, _, ExecutorWorkflowStatus::OperatorReady)
        | (ExecutorState::Operational, _, _, ExecutorWorkflowStatus::CliReady)
        | (ExecutorState::Operational, _, _, ExecutorWorkflowStatus::EngineReady) => {
            ExecutorRuntimeStatus::RunnableNow
        }
        (ExecutorState::Operational, _, _, ExecutorWorkflowStatus::InternalReady) => {
            ExecutorRuntimeStatus::RunnableWithTypedInput
        }
        (ExecutorState::Operational, _, _, ExecutorWorkflowStatus::NeedsOperatorWorkflow) => {
            ExecutorRuntimeStatus::RequiresWorkflow
        }
    }
}

fn derive_evidence_status(
    authority: ExecutorAuthorityClass,
    effect: ExecutorEffectClass,
) -> ExecutorEvidenceStatus {
    match (authority, effect) {
        (ExecutorAuthorityClass::AdvisoryOnly, _) => ExecutorEvidenceStatus::AdvisoryOnly,
        (ExecutorAuthorityClass::EvidenceReducer, _) => ExecutorEvidenceStatus::ConsumesEvidence,
        (_, ExecutorEffectClass::ReadOnlyEvidence) => ExecutorEvidenceStatus::EmitsEvidence,
        (_, ExecutorEffectClass::PlanningOnly) => ExecutorEvidenceStatus::RequiresExternalEvidence,
        (_, ExecutorEffectClass::GovernanceReadiness) => ExecutorEvidenceStatus::ConsumesEvidence,
        _ => ExecutorEvidenceStatus::EmitsReceipt,
    }
}

fn derive_maturity(
    legacy_state: ExecutorState,
    authority: ExecutorAuthorityClass,
    effect: ExecutorEffectClass,
) -> ExecutorMaturity {
    if effect == ExecutorEffectClass::ProductionLifecycle {
        ExecutorMaturity::ProductionLifecycle
    } else if authority == ExecutorAuthorityClass::AdvisoryOnly {
        ExecutorMaturity::OperationalAdvisory
    } else {
        match legacy_state {
            ExecutorState::Operational => ExecutorMaturity::Operational,
            ExecutorState::OperationalNeedsWorkflow => ExecutorMaturity::NeedsWorkflow,
            ExecutorState::ExperimentalCandidateOnly => ExecutorMaturity::OperationalCandidate,
            ExecutorState::ArchitectureOnly => ExecutorMaturity::OperationalAdvisory,
        }
    }
}

fn derive_actionable_now(surfaces: &[ExecutorSurface], operator_recipe_id: Option<&str>) -> bool {
    operator_recipe_id.is_some()
        || surfaces.contains(&ExecutorSurface::TidexCli)
        || surfaces.contains(&ExecutorSurface::TidexOperator)
        || surfaces.contains(&ExecutorSurface::Binary)
}

impl ExecutorDescriptor {
    pub fn new(draft: ExecutorDescriptorDraft<'_>) -> BrainResult<Self> {
        let workflow_status =
            derive_workflow_status(draft.state, draft.surfaces, draft.operator_recipe_id);
        let runtime_status = derive_runtime_status(
            draft.state,
            draft.authority,
            draft.effect_class,
            workflow_status,
        );
        let evidence_status = derive_evidence_status(draft.authority, draft.effect_class);
        let maturity = derive_maturity(draft.state, draft.authority, draft.effect_class);
        let production_authority = draft.authority == ExecutorAuthorityClass::LifecycleAuthority
            && draft.effect_class == ExecutorEffectClass::ProductionLifecycle;
        let mut value = Self {
            schema: "tidex.executor_descriptor/v1".into(),
            executor_id: draft.executor_id.into(),
            title: draft.title.into(),
            module_path: draft.module_path.into(),
            owner: draft.owner.into(),
            state: draft.state,
            implementation_status: ExecutorImplementationStatus::Implemented,
            runtime_status,
            workflow_status,
            evidence_status,
            maturity,
            production_authority,
            actionable_now: derive_actionable_now(draft.surfaces, draft.operator_recipe_id),
            authority: draft.authority,
            effect_class: draft.effect_class,
            surfaces: draft.surfaces.to_vec(),
            accepted_needs: draft
                .accepted_needs
                .iter()
                .map(|item| (*item).to_string())
                .collect(),
            requires: parse_artifact_list(draft.requires)?,
            produces: parse_artifact_list(draft.produces)?,
            operator_recipe_id: draft.operator_recipe_id.map(str::to_string),
            notes: draft.notes.into(),
            descriptor_sha256: Sha256Digest::zero(),
        };
        value.validate_without_digest()?;
        value.descriptor_sha256 = descriptor_digest(&value)?;
        value.validate()?;
        Ok(value)
    }

    fn validate_without_digest(&self) -> BrainResult<()> {
        if self.schema != "tidex.executor_descriptor/v1"
            || self.executor_id.trim().is_empty()
            || self.title.trim().is_empty()
            || self.module_path.trim().is_empty()
            || self.owner.trim().is_empty()
            || self.accepted_needs.is_empty()
            || self.produces.is_empty()
            || self.surfaces.is_empty()
            || self.executor_id.len() > 256
            || self.module_path.len() > 512
            || self
                .accepted_needs
                .iter()
                .any(|item| item.trim().is_empty())
            || self.produces.iter().any(|item| item.as_str().is_empty())
            || self.requires.iter().any(|item| item.as_str().is_empty())
        {
            return Err(BrainError::Invalid("executor_descriptor_invalid".into()));
        }
        if self.implementation_status == ExecutorImplementationStatus::Missing {
            return Err(BrainError::Invalid("executor_descriptor_implementation_missing".into()));
        }
        if self.actionable_now
            && matches!(
                self.runtime_status,
                ExecutorRuntimeStatus::BlockedByPolicy | ExecutorRuntimeStatus::RequiresWorkflow
            )
        {
            return Err(BrainError::Invalid("executor_actionable_runtime_mismatch".into()));
        }
        if matches!(self.effect_class, ExecutorEffectClass::ProductionLifecycle)
            && self.authority != ExecutorAuthorityClass::LifecycleAuthority
        {
            return Err(BrainError::Invalid("executor_lifecycle_authority_mismatch".into()));
        }
        if matches!(self.authority, ExecutorAuthorityClass::LifecycleAuthority)
            && !matches!(self.effect_class, ExecutorEffectClass::ProductionLifecycle)
        {
            return Err(BrainError::Invalid("executor_authority_effect_mismatch".into()));
        }
        Ok(())
    }

    pub fn validate(&self) -> BrainResult<()> {
        self.validate_without_digest()?;
        if descriptor_digest(self)? != self.descriptor_sha256 {
            return Err(BrainError::Integrity("executor_descriptor_digest_mismatch".into()));
        }
        Ok(())
    }
}

fn descriptor_digest(value: &ExecutorDescriptor) -> BrainResult<Sha256Digest> {
    let mut unsigned = value.clone();
    unsigned.descriptor_sha256 = Sha256Digest::zero();
    Ok(Sha256Digest::digest_domain(
        b"TIDEX:EXECUTOR-DESCRIPTOR:v1\0",
        &serde_json::to_vec(&unsigned)?,
    ))
}

macro_rules! desc {
    ($id:literal,$title:literal,$module:literal,$owner:literal,$state:ident,$authority:ident,$effect:ident,[$($surface:ident),+],[$($need:literal),+],[$($req:literal),*],[$($prod:literal),+],$recipe:expr,$notes:literal) => {
        ExecutorDescriptor::new(ExecutorDescriptorDraft {
            executor_id: $id,
            title: $title,
            module_path: $module,
            owner: $owner,
            state: ExecutorState::$state,
            authority: ExecutorAuthorityClass::$authority,
            effect_class: ExecutorEffectClass::$effect,
            surfaces: &[$(ExecutorSurface::$surface),+],
            accepted_needs: &[$($need),+],
            requires: &[$($req),*],
            produces: &[$($prod),+],
            operator_recipe_id: $recipe,
            notes: $notes,
        })?
    };
}

pub fn builtin_executor_catalog() -> BrainResult<Vec<ExecutorDescriptor>> {
    let values = vec![
        desc!(
            "acquisition.capture",
            "Acquire workspace",
            "content_vault::capture_to_vault",
            "content_vault",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator],
            ["source_acquisition"],
            ["declared_source_scope"],
            ["vault_receipt", "system_envelope"],
            Some("acquisition.capture"),
            "Captures declared bytes under budget; not semantic proof."
        ),
        desc!(
            "knowledge.engine",
            "Epistemic transition engine",
            "knowledge_engine::KnowledgeEngine",
            "knowledge_engine",
            Operational,
            Planner,
            PlanningOnly,
            [TidexCli, TidexOperator, InternalLibrary],
            ["knowledge_obligation", "evidence_gap", "need_refinement"],
            ["authenticated_knowledge_state"],
            ["knowledge_planning_decision"],
            Some("knowledge.plan"),
            "Authenticates persisted state and derives the next admissible invocation or terminal decision; execution evidence remains separately authenticated."
        ),
        desc!(
            "learning.active_aperture",
            "Active aperture planner",
            "active + learning_orchestrator",
            "learning_orchestrator",
            Operational,
            Planner,
            PlanningOnly,
            [CoreEngine, Binary, TidexOperator],
            ["adaptive_measurement", "uncertainty_reduction"],
            ["learning_target"],
            ["learning_plan", "aperture_receipt"],
            Some("learning.autonomous_plan"),
            "Plans and assimilates measurements; measurement execution remains separate."
        ),
        desc!(
            "brain.analyze",
            "BrainEngine skill-field analysis",
            "engine::BrainEngine::analyze",
            "engine",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [CoreEngine, Binary, TidexOperator],
            ["latent_structure_analysis", "skill_field_reconstruction"],
            ["authenticated_observations"],
            ["brain_analysis_receipt"],
            Some("analysis.brain"),
            "Runs the canonical engine analysis chain."
        ),
        desc!(
            "brain.sleep",
            "Evidence-bound consolidation",
            "engine::BrainEngine::sleep_cycle",
            "engine",
            Operational,
            Executor,
            CandidateArtifact,
            [CoreEngine, Binary, TidexOperator],
            ["consolidation", "memory_update"],
            ["new_evidence", "sleep_policy"],
            ["sleep_receipt", "consolidated_state"],
            Some("runtime.sleep"),
            "Consolidation is governed; not a time-based cron by itself."
        ),
        desc!(
            "residency.decide",
            "Capability residency decision",
            "residency_decision::ResidencyDecisionAuthority",
            "residency_decision",
            Operational,
            Planner,
            PlanningOnly,
            [TidexCli, TidexOperator, InternalLibrary],
            ["decide_residency", "software_vs_weights_vs_hybrid"],
            [
                "authority_instance",
                "authenticated_precommit_reference",
                "capability_bundle",
                "knowledge_state"
            ],
            ["residency_decision", "residency_decision_reference"],
            Some("residency.decide"),
            "Executes the existing residency authority from an authenticated precommit; it classifies residence only and never authorizes promotion."
        ),
        desc!(
            "numerical.evolve",
            "Numerical evolution",
            "numerical_evolution::NumericalEvolutionEngine",
            "numerical_evolution",
            Operational,
            Executor,
            CandidateArtifact,
            [TidexCli, TidexOperator, InternalLibrary],
            ["numerical_solution_insufficient", "solver_selection"],
            [
                "training_problem",
                "holdout_groups",
                "explicit_governance_policy"
            ],
            ["numerical_evolution_receipt", "candidate_solution"],
            Some("numerical.evolve"),
            "Runs an ordered in-memory evolution campaign under explicit policy; candidate evidence never authorizes promotion or production."
        ),
        desc!(
            "procedural.memory",
            "Procedural memory advisor",
            "procedural_memory",
            "procedural_memory",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [CoreEngine, InternalLibrary],
            ["strategy_reuse", "solver_advice"],
            ["past_attempts"],
            ["advisory_strategy"],
            None,
            "Advisory only; must never authorize execution or promotion."
        ),
        desc!(
            "learning.cycle",
            "Persistent adaptive learning cycle",
            "learning_orchestrator",
            "learning_orchestrator",
            Operational,
            Planner,
            PlanningOnly,
            [Binary, InternalLibrary],
            [
                "start_learning",
                "next_aperture",
                "assimilate_evidence",
                "inspect_learning_session"
            ],
            [
                "learning_target",
                "adaptive_policy",
                "authenticated_experiment_evidence"
            ],
            ["adaptive_learning_receipt"],
            None,
            "Persistent receipt-chained adaptive cycle exists in adaptive-learning-cycle; it plans and assimilates evidence but does not promote by itself."
        ),
        desc!(
            "learning.finalize",
            "Governed learning finalization",
            "learning_finalization + BrainEngine::commit_finalized_learning_session",
            "learning_finalization",
            Operational,
            Executor,
            LifecycleOperation,
            [CoreEngine, Binary, InternalLibrary],
            ["finalize_completed_learning_session"],
            [
                "completed_adaptive_session",
                "sealed_representation_evidence"
            ],
            ["learning_finalization_receipt"],
            None,
            "Replays adaptive and representation evidence before engine commit; it is not AdapterBank production activation authority."
        ),
        desc!(
            "learning.controller",
            "Persisted learned controller",
            "learned_controller",
            "learned_controller",
            Operational,
            Executor,
            RuntimeIntervention,
            [CoreEngine, Binary, InternalLibrary],
            [
                "train_controller",
                "inspect_controller",
                "compose_controller"
            ],
            [
                "authenticated_controller_dataset",
                "controller_policy",
                "promoted_state_binding"
            ],
            [
                "learned_controller_receipt",
                "recorded_controller_execution"
            ],
            None,
            "Controller training and runtime composition are receipt-bound and OOD guarded; no free-form coefficient authority is accepted."
        ),
        desc!(
            "representation.evidence",
            "Sealed representation evidence",
            "representation_evidence::record_representation_evidence",
            "representation_evidence",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [Binary, InternalLibrary],
            ["seal_representation_observations"],
            ["sealed_install_request"],
            ["representation_evidence_receipt"],
            None,
            "Records confined representation evidence for later learning finalization; it does not promote observations by itself."
        ),
        desc!(
            "causal.credit",
            "Causal credit reducer",
            "causal_credit::estimate_causal_credit",
            "causal_credit",
            Operational,
            EvidenceReducer,
            ReadOnlyEvidence,
            [CoreEngine, InternalLibrary],
            ["counterfactual_credit", "causal_priority"],
            ["counterfactual_evaluations"],
            ["causal_credit_report"],
            None,
            "Canonical causal reducer is used by engine/sleep paths; it consumes measured counterfactual evidence and has no promotion authority."
        ),
        desc!(
            "cognitive.field",
            "Dynamic cognitive field",
            "cognitive_field::DynamicCognitiveField",
            "cognitive_field",
            Operational,
            Planner,
            PlanningOnly,
            [CoreEngine, InternalLibrary],
            ["field_dynamics", "evidence_routing"],
            ["skill_fields", "curvature", "causal_credit_report"],
            ["cognitive_field_state", "field_routing_decision"],
            None,
            "Engine-integrated dynamics/routing layer; its routing decision remains planning evidence, not execution authority."
        ),
        desc!(
            "dual.space",
            "Dual-space representation analysis",
            "dual_space::analyze_dual_space",
            "dual_space",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [CoreEngine, InternalLibrary],
            ["parameter_representation_alignment"],
            ["skill_fields", "representation_observations"],
            ["dual_space_analysis"],
            None,
            "Runs inside BrainEngine analysis and produces representation/parameter agreement evidence only."
        ),
        desc!(
            "trajectory.sbas",
            "SBAS trajectory reconstruction",
            "sbas::reconstruct_trajectory",
            "sbas",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [CoreEngine, InternalLibrary],
            ["trajectory_reconstruction", "cycle_closure"],
            ["delta_observations"],
            ["sbas_trajectory"],
            None,
            "Canonical trajectory reconstruction is already part of BrainEngine analysis; diagnostic evidence only."
        ),
        desc!(
            "tomography.weight",
            "Temporal weight tomography",
            "weight_tomography",
            "weight_tomography",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [CoreEngine, InternalLibrary],
            ["temporal_weight_dynamics", "pathology_veto"],
            ["generation_ordered_delta_history"],
            ["weight_tomography_observation", "tomography_gate_decision"],
            None,
            "Bounded temporal diagnostic over measured deltas; may veto pathological trajectories but cannot promote."
        ),
        desc!(
            "temporal.tracking",
            "Temporal coherence tracking",
            "temporal_tracking",
            "temporal_tracking",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [CoreEngine, InternalLibrary],
            [
                "phase_correlation",
                "sbas_time_series",
                "persistent_scatterers"
            ],
            ["weight_snapshots_or_differentials"],
            ["temporal_tracking_evidence", "temporal_advisory_signal"],
            None,
            "Operational advisory engine reused by SAR/topology paths; it does not authorize execution or promotion by itself."
        ),
        desc!(
            "transport.functional",
            "Functional transplant map",
            "transport::learn_functional_transplant",
            "transport",
            Operational,
            Executor,
            CandidateArtifact,
            [CoreEngine, InternalLibrary],
            ["functional_to_receiver_transport"],
            ["source_functional_anchors", "target_coordinate_anchors"],
            ["functional_transport_map"],
            None,
            "Canonical transport is used by receiver compilation; its output is a bounded candidate map, not behavioral transfer proof."
        ),
        desc!(
            "transport.relational",
            "Relational transport map",
            "transport::learn_relational_transport",
            "transport",
            Operational,
            Executor,
            CandidateArtifact,
            [CoreEngine, InternalLibrary],
            ["relational_cross_space_transport"],
            ["source_anchors", "target_anchors"],
            ["relational_transport_map"],
            None,
            "Learns a validated relational map with holdout diagnostics; it does not authorize model mutation or promotion."
        ),
        desc!(
            "ledger.verify",
            "Verified governance ledger",
            "ledger::verified_v2_snapshot",
            "ledger",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [Binary, InternalLibrary],
            ["ledger_diagnostics", "ledger_replay"],
            ["private_authority_root"],
            ["verified_ledger_snapshot"],
            None,
            "Verifies event-chain integrity and exposes the authenticated head; diagnostic only."
        ),
        desc!(
            "tomography.skill_fields",
            "Skill-field tomography",
            "tomography + persistent + identifiability + dual_space",
            "engine",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [CoreEngine, TidexCli, TidexOperator, InternalLibrary],
            ["functional_structure_reconstruction"],
            ["authenticated_observed_deltas"],
            [
                "skill_fields",
                "identifiability_report",
                "tomography_receipt"
            ],
            Some("analysis.tomography"),
            "Exposed through the canonical BrainEngine analysis chain; produces evidence only and never authorizes production."
        ),
        desc!(
            "protected.map",
            "Protected cortex map",
            "protected_map",
            "protected_map",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [CoreEngine, TidexCli, TidexOperator, Binary],
            ["protect_invariants", "damage_constraint"],
            ["authenticated_sensitivity_artifacts"],
            ["protected_map", "protected_map_receipt"],
            Some("analysis.protected_map"),
            "Builds and persists the canonical protected-cortex map from authenticated sensitivity evidence; never authorizes promotion."
        ),
        desc!(
            "pythagoras.geometry",
            "Pythagoras topology/geometry",
            "pythagoras_topology",
            "pythagoras_topology",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator, InternalLibrary],
            ["geometry_diagnostic", "topology_check"],
            ["candidate_vector", "manifold_samples"],
            ["pythagoras_report", "topological_manifold_report"],
            Some("analysis.pythagoras"),
            "Typed geometry/topology diagnostic exposed through tidex; it remains evidence-only and is not a learning or promotion authority."
        ),
        desc!(
            "trust.region",
            "Trust-region contraction",
            "trust_region",
            "trust_region",
            Operational,
            Executor,
            CandidateArtifact,
            [CoreEngine, Binary],
            ["risk_bounded_delta"],
            ["candidate_delta", "risk_budget"],
            ["bounded_delta", "risk_receipt"],
            None,
            "Constrains proposed changes; does not prove behavioral improvement."
        ),
        desc!(
            "discovery.capabilities",
            "Capability discovery",
            "receiver::capability_discovery::CapabilityDiscoveryRequest::execute",
            "receiver/capability_discovery",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator, InternalLibrary],
            ["capability_discovery"],
            ["request"],
            ["capability_discovery_report"],
            Some("discovery.capabilities"),
            "Executes the canonical evidence-based capability discovery request; reports evidence readiness but does not mint CapabilityIR or promotion authority."
        ),
        desc!(
            "benchmark.receiver_response",
            "Receiver response benchmark",
            "receiver::receiver_compiler::benchmark_receiver_signature",
            "receiver/receiver_compiler",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator, InternalLibrary],
            ["receiver_response_benchmark"],
            ["benchmark"],
            ["receiver_signature_compilation"],
            Some("benchmark.response"),
            "Runs the canonical numerical receiver-signature benchmark and returns its measured compilation report; no mutation or promotion authority."
        ),
        desc!(
            "benchmark.receiver_basis",
            "Receiver basis benchmark",
            "receiver::receiver_compiler::benchmark_receiver_basis",
            "receiver/receiver_compiler",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator, InternalLibrary],
            ["receiver_basis_benchmark"],
            ["benchmark"],
            ["receiver_basis_benchmark_report"],
            Some("benchmark.receiver_basis"),
            "Runs the canonical receiver-basis decomposition over caller-supplied authenticated calibration data; evidence only."
        ),
        desc!(
            "benchmark.receiver_portability",
            "Receiver portability benchmark",
            "receiver::receiver_compiler::benchmark_receiver_portability_leave_one_out",
            "receiver/receiver_compiler",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator, InternalLibrary],
            ["receiver_portability_benchmark"],
            ["benchmark"],
            ["receiver_portability_benchmark_report"],
            Some("benchmark.portability"),
            "Runs the canonical leave-one-out receiver portability benchmark; evidence only and workspace-independent."
        ),
        desc!(
            "receiver.profile",
            "Physical receiver profiling",
            "model_adaptation::profile_receiver_model",
            "model_adaptation",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator],
            ["receiver_profile"],
            ["checkpoint", "config", "tokenizer"],
            ["receiver_model_profile"],
            Some("receiver.profile"),
            "Authenticates physical receiver geometry."
        ),
        desc!(
            "receiver.normalize_sharded",
            "Normalize sharded SafeTensors",
            "weight_actuator::normalize_sharded_safetensors",
            "weight_actuator",
            Operational,
            Executor,
            CandidateArtifact,
            [TidexCli, TidexOperator],
            ["normalize_checkpoint"],
            ["hf_weight_map", "shards"],
            ["single_safetensors_checkpoint"],
            Some("receiver.normalize_sharded"),
            "Structural checkpoint operation; not behavioral evidence."
        ),
        desc!(
            "receiver.freeze_compiler",
            "Freeze receiver compiler",
            "receiver_compiler::freeze_receiver_compiler",
            "receiver_compiler",
            Operational,
            Planner,
            CandidateArtifact,
            [TidexCli, TidexOperator],
            ["freeze_calibration"],
            ["calibration_cases", "protection", "risk_policy"],
            ["frozen_receiver_compiler"],
            Some("receiver.freeze_compiler"),
            "Commits calibration before held-out target compilation."
        ),
        desc!(
            "receiver.verify_frozen",
            "Verify frozen receiver compiler",
            "receiver::receiver_compiler::FrozenReceiverCompiler::verify",
            "receiver/receiver_compiler",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator, InternalLibrary],
            ["verify_frozen_compiler"],
            ["frozen_receiver_compiler"],
            ["frozen_receiver_compiler_verification"],
            Some("receiver.verify_frozen"),
            "Replay-verifies the frozen compiler commitment without refitting or mutation."
        ),
        desc!(
            "receiver.compile_universal",
            "Compile held-out capability",
            "universal_capability_compiler",
            "universal_capability_compiler",
            ExperimentalCandidateOnly,
            Executor,
            CandidateArtifact,
            [TidexCli, TidexOperator],
            ["compile_capability_to_receiver"],
            ["frozen_receiver_compiler", "capability_ir"],
            ["universal_compilation_receipt"],
            Some("compile.universal"),
            "Candidate-only; not universal LLM transfer proof."
        ),
        desc!(
            "compile.universal_plan",
            "Plan universal shadow",
            "materialization::universal_capability_compiler::execute_universal_capability_shadow_plan",
            "materialization/universal_capability_compiler",
            ExperimentalCandidateOnly,
            Planner,
            CandidateArtifact,
            [TidexCli, TidexOperator, InternalLibrary],
            ["plan_universal_shadow"],
            ["universal_plan_request"],
            ["universal_plan_receipt"],
            Some("compile.universal_plan"),
            "Executes the existing bound universal shadow planner; result remains candidate-only and requires replay/authentication downstream."
        ),
        desc!(
            "materialize.compiled",
            "Materialize compiled checkpoint",
            "materialization_pipeline::materialize_compiled_checkpoint",
            "materialization_pipeline",
            ExperimentalCandidateOnly,
            Executor,
            PhysicalMaterialization,
            [TidexCli, TidexOperator],
            ["physical_realization"],
            ["materialization_request"],
            ["candidate_checkpoint", "materialization_receipt"],
            Some("materialize.compiled"),
            "Writes candidate checkpoints; does not authorize production."
        ),
        desc!(
            "materialize.verify",
            "Verify compiled checkpoint",
            "materialization::materialization_pipeline::authenticate_compiled_checkpoint",
            "materialization/materialization_pipeline",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator, InternalLibrary],
            ["verify_compiled_checkpoint"],
            ["materialization_receipt"],
            ["materialization_verification"],
            Some("materialize.verify"),
            "Replay-authenticates the persisted materialization receipt and physical checkpoint arithmetic; read-only."
        ),
        desc!(
            "materialize.shadow_dense",
            "Dense shadow materialization",
            "dense_shadow_materializer::materialize_replayed_dense_delta_shadow",
            "dense_shadow_materializer",
            ExperimentalCandidateOnly,
            Executor,
            CandidateArtifact,
            [TidexCli, TidexOperator, InternalLibrary],
            ["dense_shadow_candidate"],
            [
                "universal_plan_request",
                "universal_plan_receipt",
                "receiver_layout"
            ],
            ["dense_shadow_artifact"],
            Some("materialize.shadow_dense"),
            "Replays a bound universal shadow plan into a dense candidate only; no activation or promotion authority."
        ),
        desc!(
            "materialize.shadow_low_rank",
            "Low-rank shadow materialization",
            "low_rank_shadow_materializer::materialize_replayed_low_rank_shadow",
            "low_rank_shadow_materializer",
            ExperimentalCandidateOnly,
            Executor,
            CandidateArtifact,
            [TidexCli, TidexOperator, InternalLibrary],
            ["low_rank_shadow_candidate"],
            [
                "universal_plan_request",
                "universal_plan_receipt",
                "receiver_layout",
                "low_rank_policy"
            ],
            ["low_rank_shadow_artifact"],
            Some("materialize.shadow_low_rank"),
            "Bounded low-rank approximation with replay evidence; lossy candidates do not inherit compiled-candidate validation."
        ),
        desc!(
            "materialize.shadow_sparse",
            "Sparse shadow materialization",
            "sparse_shadow_materializer::materialize_replayed_sparse_shadow",
            "sparse_shadow_materializer",
            ExperimentalCandidateOnly,
            Executor,
            CandidateArtifact,
            [TidexCli, TidexOperator, InternalLibrary],
            ["sparse_shadow_candidate"],
            [
                "universal_plan_request",
                "universal_plan_receipt",
                "receiver_layout",
                "sparse_policy"
            ],
            ["sparse_shadow_artifact"],
            Some("materialize.shadow_sparse"),
            "Bounded sparse approximation with replay evidence; remains a candidate-only representation."
        ),
        desc!(
            "materialize.shadow_steering",
            "Activation-steering shadow materialization",
            "activation_steering_materializer::materialize_replayed_activation_steering_shadow",
            "activation_steering_materializer",
            ExperimentalCandidateOnly,
            Executor,
            RuntimeIntervention,
            [TidexCli, TidexOperator, InternalLibrary],
            ["activation_steering_shadow_candidate"],
            [
                "universal_plan_request",
                "universal_plan_receipt",
                "receiver_layout",
                "steering_layout",
                "steering_policy"
            ],
            ["activation_steering_shadow_artifact"],
            Some("materialize.shadow_steering"),
            "Produces a replayable steering intervention candidate; no runtime hook is installed and no production authority is granted."
        ),
        desc!(
            "materialize.selector",
            "Select materialization backend",
            "materialization_selector",
            "materialization_selector",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexCli, TidexOperator],
            ["choose_materialization_backend"],
            ["measured_backend_metrics"],
            ["backend_ranking"],
            Some("selection.backend"),
            "Ranks supplied metrics; it does not attest their origin."
        ),
        desc!(
            "shadow.evaluate",
            "Isolated shadow evaluation",
            "shadow_evaluation",
            "shadow_evaluation",
            ExperimentalCandidateOnly,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator],
            ["shadow_validation"],
            ["runner", "bundle", "limits"],
            ["shadow_receipt"],
            Some("evaluation.shadow"),
            "Isolation exists; evaluators still need to perform real measurements."
        ),
        desc!(
            "universality.reduce",
            "Universality evidence reducer",
            "universality_evidence",
            "universality_evidence",
            Operational,
            EvidenceReducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator],
            ["universality_reduction"],
            ["heldout_trials"],
            ["universality_receipt"],
            Some("evidence.universality"),
            "Reduces submitted trials; does not execute them."
        ),
        desc!(
            "promotion.readiness",
            "Universal promotion readiness",
            "universal_promotion_gate",
            "universal_promotion_gate",
            Operational,
            ReadinessGate,
            GovernanceReadiness,
            [TidexCli, TidexOperator],
            ["promotion_readiness"],
            ["shadow", "selection", "universality"],
            ["readiness_decision"],
            Some("promotion.readiness"),
            "Never emits activation authority."
        ),
        desc!(
            "adapter.bank",
            "AdapterBank lifecycle authority",
            "adapter_bank::AdapterBank",
            "adapter_bank",
            Operational,
            LifecycleAuthority,
            ProductionLifecycle,
            [TidexCli, TidexOperator],
            [
                "adapter_import",
                "compose",
                "authorize",
                "activate",
                "revoke",
                "rollback"
            ],
            ["governed_authorization"],
            ["adapter_bank_revision"],
            None,
            "Only production lifecycle authority for adapters; concrete recipes bind to the specific lifecycle entrypoints below."
        ),
        desc!(
            "adapter.import",
            "Import adapter candidate",
            "adapter_bank::AdapterBank::import",
            "adapter_bank",
            Operational,
            Executor,
            LifecycleOperation,
            [TidexCli, TidexOperator],
            ["adapter_import"],
            ["adapter_import_request", "authenticated_candidate"],
            ["adapter_manifest", "adapter_bank_revision"],
            Some("adapter.import"),
            "Concrete AdapterBank entrypoint; mutates the bank through the canonical authority and emits a new revision."
        ),
        desc!(
            "adapter.compose",
            "Compose adapters exactly",
            "adapter_bank::AdapterBank::compose_exact",
            "adapter_bank",
            Operational,
            Executor,
            LifecycleOperation,
            [TidexCli, TidexOperator],
            ["compose_adapters"],
            ["ordered_adapter_ids", "adapter_bank_revision"],
            ["adapter_composition_manifest", "adapter_bank_revision"],
            Some("adapter.compose"),
            "Exact ordered adapter composition; no SVD, pruning or rank truncation."
        ),
        desc!(
            "adapter.materialize",
            "Materialize adapter candidate",
            "adapter_bank::AdapterBank::materialize_candidate",
            "adapter_bank",
            Operational,
            Executor,
            CandidateArtifact,
            [TidexCli, TidexOperator],
            ["materialize_adapter_candidate"],
            ["adapter_resolution", "materialization_request"],
            [
                "candidate_adapter_artifact",
                "adapter_materialization_receipt"
            ],
            Some("adapter.materialize"),
            "Candidate-only adapter materialization; does not activate production."
        ),
        desc!(
            "adapter.authorize",
            "Authorize governed adapter promotion",
            "adapter_bank::AdapterBank::authorize_governed_promotion_request",
            "adapter_bank",
            Operational,
            Executor,
            GovernanceReadiness,
            [TidexCli, TidexOperator],
            ["authorize_adapter_promotion"],
            ["sealed_gate_witnesses", "adapter_bank_revision"],
            ["current_state_promotion_permit"],
            Some("adapter.authorize"),
            "Mints a governed permit bound to current bank state; still distinct from activation."
        ),
        desc!(
            "adapter.activate",
            "Activate adapter with permit",
            "adapter_bank::AdapterBank::activate",
            "adapter_bank",
            Operational,
            Executor,
            LifecycleOperation,
            [TidexCli, TidexOperator],
            ["activate_adapter"],
            ["current_state_promotion_permit"],
            ["active_adapter_selection", "adapter_bank_revision"],
            Some("adapter.activate"),
            "Production-changing entrypoint delegated to AdapterBank; requires a governed permit and emits a revision."
        ),
        desc!(
            "adapter.revoke",
            "Revoke adapter transitively",
            "adapter_bank::AdapterBank::revoke",
            "adapter_bank",
            Operational,
            Executor,
            LifecycleOperation,
            [TidexCli, TidexOperator],
            ["revoke_adapter"],
            ["revocation_request", "adapter_bank_revision"],
            ["sticky_revocation", "adapter_bank_revision"],
            Some("adapter.revoke"),
            "Sticky governed revocation of an adapter and dependent compositions."
        ),
        desc!(
            "adapter.rollback",
            "Rollback AdapterBank forward",
            "adapter_bank::AdapterBank::rollback",
            "adapter_bank",
            Operational,
            Executor,
            LifecycleOperation,
            [TidexCli, TidexOperator],
            ["rollback_adapter_bank"],
            ["rollback_request", "verified_history"],
            ["forward_rollback_revision"],
            Some("adapter.rollback"),
            "Publishes a new forward revision representing rollback; no history rewrite."
        ),
        desc!(
            "adapter.status",
            "Verify AdapterBank history",
            "adapter_bank::AdapterBank::verify_history",
            "adapter_bank",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexCli, TidexOperator],
            ["verify_adapter_bank"],
            ["adapter_bank_history"],
            ["adapter_bank_status_receipt"],
            Some("adapter.status"),
            "Read-only verification of the complete hash-linked AdapterBank history."
        ),
        desc!(
            "cross_model.probe_runtime",
            "Probe model runtime access",
            "tidex_operator_runner::probe_runtime",
            "cross_model/models",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexOperator, Binary],
            ["runtime_capability_probe"],
            ["hf_snapshot"],
            ["runtime_access_profile"],
            Some("operator.direct_runner"),
            "Actually opens the checkpoint and reports supported access classes."
        ),
        desc!(
            "cross_model.evaluate",
            "Behavioral LLM evaluation",
            "cross_model::discovery::evaluate_model",
            "cross_model/discovery",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexOperator, Binary],
            ["behavioral_evaluation"],
            ["llm_runtime", "behavioral_benchmark"],
            ["model_evaluation"],
            Some("operator.direct_runner"),
            "Executes real model generations and verifier scoring."
        ),
        desc!(
            "cross_model.discovery",
            "Multi-LLM discovery cycle",
            "plasticity_engine::run_discovery_pipeline",
            "cross_model/plasticity_engine",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexOperator, CrossModelDaemon],
            ["compare_models", "detect_gaps"],
            ["two_or_more_llms", "behavioral_benchmark"],
            ["discovery_cycle", "gaps", "proposals"],
            Some("cross_model.discovery_cycle"),
            "Produces gaps/proposals, not transfer success."
        ),
        desc!(
            "cross_model.extract_steering",
            "Hierarchical steering extraction",
            "cross_model::extraction::HierarchicalSteeringExtractor",
            "cross_model/extraction",
            ExperimentalCandidateOnly,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexOperator, Binary],
            ["extract_internal_representation"],
            [
                "positive_examples",
                "negative_examples",
                "internal_activations"
            ],
            ["steering_vector_candidate"],
            Some("operator.direct_runner"),
            "Representation candidate; requires behavioral validation."
        ),
        desc!(
            "cross_model.align",
            "Cross-model activation alignment",
            "cross_model::extraction::CrossModelAligner",
            "cross_model/extraction",
            ExperimentalCandidateOnly,
            Executor,
            CandidateArtifact,
            [TidexOperator, Binary],
            ["align_representations"],
            ["paired_activations", "validation_prompts"],
            ["alignment_calibration"],
            Some("operator.direct_runner"),
            "Calibrated map; fails closed on residual."
        ),
        desc!(
            "cross_model.counterfactual",
            "Counterfactual analysis",
            "cross_model::extraction::CounterfactualAnalyzer",
            "cross_model/extraction",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexOperator, Binary],
            ["counterfactual_effect"],
            ["scenario", "verifier"],
            ["counterfactual_result"],
            Some("operator.direct_runner"),
            "Measures response changes; causality depends on scenario design."
        ),
        desc!(
            "cross_model.transfer_steering",
            "Activation steering transfer experiment",
            "tidex_operator_runner::ActivationTransferExperiment",
            "cross_model/extraction",
            ExperimentalCandidateOnly,
            Executor,
            RuntimeIntervention,
            [TidexOperator, Binary],
            ["activation_transfer_experiment"],
            ["source", "target", "extraction", "alignment", "benchmark"],
            ["baseline_intervened_restored_report"],
            Some("operator.direct_runner"),
            "Experimental runtime hook with explicit restoration check."
        ),
        desc!(
            "cross_model.nnsight",
            "Deep instrumentation",
            "HfTransformersModel::deep_instrumentation",
            "cross_model/models",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexOperator, Binary],
            ["deep_instrumentation"],
            [
                "runtime_profile.deep_instrumentation",
                "module_path",
                "prompt"
            ],
            ["deep_instrumentation_evidence"],
            Some("operator.direct_runner"),
            "Operational when the selected model profile exposes deep_instrumentation=true; blocked per-model otherwise."
        ),
        desc!(
            "cross_model.sae",
            "Sparse autoencoder analysis",
            "HfTransformersModel::sparse_autoencoder_analysis",
            "cross_model/models",
            Operational,
            EvidenceProducer,
            ReadOnlyEvidence,
            [TidexOperator, Binary],
            ["sae_feature_analysis"],
            [
                "runtime_profile.sparse_autoencoder_analysis",
                "single_safetensors_checkpoint",
                "module_path",
                "prompt"
            ],
            ["sparse_feature_evidence"],
            Some("operator.direct_runner"),
            "Operational when the selected model profile exposes sparse_autoencoder_analysis=true and a bound local SAE dictionary (sae.safetensors + config.json) is supplied. This applies a frozen encoder/decoder to measured activations; it does not train."
        ),
        desc!(
            "plasticity.bcm",
            "BCM metaplasticity",
            "cross_model::plasticity::BCMMetaplasticity",
            "cross_model/plasticity",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexOperator, InternalLibrary],
            ["adaptive_threshold"],
            ["activity_observation"],
            ["threshold_signal"],
            None,
            "Fed by config/plasticity.toml plus durable operator/plasticity/controller_state.json through OperatorPlasticityAdvice v2; advisory numerical controller only — not PlasticityEngine and never promotion authority."
        ),
        desc!(
            "plasticity.eligibility",
            "Eligibility traces",
            "cross_model::plasticity::EligibilityTraces",
            "cross_model/plasticity",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexOperator, InternalLibrary],
            ["temporal_credit"],
            ["job_outcomes", "activation_events"],
            ["eligibility_signal"],
            None,
            "Consumes measured Operator outcomes with durable controller state under operator/plasticity/; advisory credit only, not PlasticityEngine."
        ),
        desc!(
            "plasticity.neuromodulation",
            "Neuromodulation",
            "cross_model::plasticity::Neuromodulation",
            "cross_model/plasticity",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexOperator, InternalLibrary],
            ["adaptation_rate_modulation"],
            ["reward", "novelty", "stability"],
            ["modulated_rate"],
            None,
            "Combines measured Operator evaluation signals and modulates BCM/eligibility/content/routing learning rates in plasticity v2; cannot alter hard gates or authorize production."
        ),
        desc!(
            "plasticity.routing",
            "Routing plasticity",
            "cross_model::plasticity::RoutingPlasticity",
            "cross_model/plasticity",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexOperator, CoreEngine, InternalLibrary],
            ["choose_model_or_route"],
            ["catalog", "outcomes"],
            ["routing_decision"],
            None,
            "Consumes measured Operator outcomes with newer-valid evidence merge and durable routing history; emits advisory routing only."
        ),
        desc!(
            "plasticity.content",
            "Content plasticity",
            "cross_model::plasticity::ContentPlasticity",
            "cross_model/plasticity",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexOperator, InternalLibrary],
            ["content_drift"],
            ["fact/source_observations"],
            ["content_confidence_update"],
            None,
            "Runs inside the evidence-bound plasticity v2 projection with durable state under operator/plasticity/; advisory only."
        ),
        desc!(
            "plasticity.pi",
            "PI controller",
            "cross_model::plasticity::PIController",
            "cross_model/plasticity",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexOperator, InternalLibrary],
            ["control_adaptation_pressure"],
            ["setpoint", "measurement"],
            ["bounded_control_signal"],
            None,
            "Runs against measured score distributions with durable PI integral under operator/plasticity/; bounded advisory control only."
        ),
        desc!(
            "plasticity.elo",
            "ELO result ranking",
            "cross_model::plasticity::ELOSystem",
            "cross_model/plasticity",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexOperator, InternalLibrary],
            ["rank_models_or_strategies"],
            ["paired_outcomes"],
            ["elo_rating_update"],
            None,
            "Consumes paired measured Operator outcomes with durable ratings under operator/plasticity/; comparisons are advisory and never promotion evidence by themselves."
        ),
        desc!(
            "coevolution.loop",
            "Co-evolution bidirectional loop",
            "cross_model::co_evolution::BidirectionalLoop",
            "cross_model/co_evolution",
            Operational,
            AdvisoryOnly,
            AdvisorySignal,
            [TidexOperator, InternalLibrary],
            ["analyze_learning_dynamics", "steer_next_cycle"],
            ["cycle_history"],
            ["coevolution_progress", "coevolution_directive"],
            None,
            "Operational advisory loop: seals causally filtered discovery+intervention history, plans the next discovery/transfer job, and steers durable routing/PI under operator/plasticity/; never production_authority."
        ),
        desc!(
            "consensus.builder",
            "Consensus builder",
            "cross_model::co_evolution::ConsensusBuilder",
            "cross_model/co_evolution",
            ArchitectureOnly,
            AdvisoryOnly,
            AdvisorySignal,
            [InternalLibrary],
            ["governance_quorum"],
            ["policy_required_votes"],
            ["consensus_state"],
            None,
            "Use only when policy requires quorum."
        ),
        desc!(
            "operator.jobs",
            "TIDE-X operator job supervisor",
            "operator",
            "operator",
            Operational,
            Executor,
            PlanningOnly,
            [TidexOperator, TidexCli],
            ["operator_job"],
            ["executor_descriptor", "request"],
            ["operator_job_record", "operator_evidence_receipt"],
            None,
            "Supervises jobs, cancellation, recovery and receipts; never authorizes production."
        ),
    ];
    validate_catalog(values)
}

pub fn validate_catalog(
    mut values: Vec<ExecutorDescriptor>,
) -> BrainResult<Vec<ExecutorDescriptor>> {
    let mut ids = BTreeSet::new();
    for value in &mut values {
        value.implementation_status = ExecutorImplementationStatus::Implemented;
        value.descriptor_sha256 = Sha256Digest::zero();
        value.descriptor_sha256 = descriptor_digest(value)?;
        value.validate()?;
        if !ids.insert(value.executor_id.clone()) {
            return Err(BrainError::Integrity("executor_descriptor_duplicate".into()));
        }
    }
    values.sort_by(|left, right| left.executor_id.cmp(&right.executor_id));
    Ok(values)
}

pub fn executor_catalog() -> BrainResult<Vec<ExecutorDescriptor>> {
    builtin_executor_catalog()
}

/// Builtin catalog plus durable promotions under `tidex_home`.
pub fn executor_catalog_at(tidex_home: &std::path::Path) -> BrainResult<Vec<ExecutorDescriptor>> {
    let mut values = builtin_executor_catalog()?;
    let promoted = crate::operator::promoted_executor_catalog::load_promoted_executor_descriptors(tidex_home)?;
    values.extend(promoted);
    validate_catalog(values)
}

pub fn executor_by_id(id: &str) -> BrainResult<ExecutorDescriptor> {
    executor_catalog()?
        .into_iter()
        .find(|entry| entry.executor_id == id)
        .ok_or_else(|| BrainError::Invalid("executor_descriptor_unknown".into()))
}

pub fn executor_by_id_in(
    tidex_home: &std::path::Path,
    id: &str,
) -> BrainResult<ExecutorDescriptor> {
    executor_catalog_at(tidex_home)?
        .into_iter()
        .find(|entry| entry.executor_id == id)
        .ok_or_else(|| BrainError::Invalid("executor_descriptor_unknown".into()))
}

pub fn executor_id_for_operation_at(
    tidex_home: &std::path::Path,
    operation: &str,
) -> BrainResult<Option<String>> {
    if let Some(id) = executor_id_for_direct_operation(operation) {
        return Ok(Some(id.to_string()));
    }
    crate::operator::promoted_executor_catalog::promoted_operation_executor_id(tidex_home, operation)
}

pub fn executors_for_operator_recipe(recipe_id: &str) -> BrainResult<Vec<ExecutorDescriptor>> {
    Ok(executor_catalog()?
        .into_iter()
        .filter(|entry| entry.operator_recipe_id.as_deref() == Some(recipe_id))
        .collect())
}

pub fn executor_for_operator_recipe(recipe_id: &str) -> BrainResult<Option<ExecutorDescriptor>> {
    let matches = executors_for_operator_recipe(recipe_id)?;
    if matches.len() == 1 {
        Ok(matches.into_iter().next())
    } else {
        Ok(None)
    }
}

pub fn executor_id_for_direct_operation(operation: &str) -> Option<&'static str> {
    match operation {
        "behavioral_discovery" => Some("cross_model.discovery"),
        "probe_runtime" => Some("cross_model.probe_runtime"),
        "behavioral_evaluation" => Some("cross_model.evaluate"),
        "extract_capability" => Some("cross_model.extract_steering"),
        "deep_instrumentation" => Some("cross_model.nnsight"),
        "sparse_autoencoder_analysis" => Some("cross_model.sae"),
        "counterfactual_analysis" => Some("cross_model.counterfactual"),
        "calibrate_alignment" => Some("cross_model.align"),
        "activation_transfer_experiment" => Some("cross_model.transfer_steering"),
        "generate_behavioral_dataset" => Some("cross_model.evaluate"),
        _ => None,
    }
}

pub fn direct_runner_operation_bindings() -> &'static [(&'static str, &'static str)] {
    &[
        ("probe_runtime", "cross_model.probe_runtime"),
        ("behavioral_evaluation", "cross_model.evaluate"),
        ("extract_capability", "cross_model.extract_steering"),
        ("deep_instrumentation", "cross_model.nnsight"),
        ("sparse_autoencoder_analysis", "cross_model.sae"),
        ("counterfactual_analysis", "cross_model.counterfactual"),
        ("calibrate_alignment", "cross_model.align"),
        ("activation_transfer_experiment", "cross_model.transfer_steering"),
        ("generate_behavioral_dataset", "cross_model.evaluate"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_valid_and_unique() {
        let values = executor_catalog().unwrap();
        assert!(values.len() >= 30);
        let ids = values
            .iter()
            .map(|entry| entry.executor_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), values.len());
        assert!(ids.contains("adapter.bank"));
        assert!(ids.contains("cross_model.evaluate"));
        assert!(ids.contains("knowledge.engine"));
        assert!(values
            .iter()
            .all(|entry| entry.implementation_status == ExecutorImplementationStatus::Implemented));
        assert_eq!(
            values
                .iter()
                .find(|entry| entry.executor_id == "plasticity.bcm")
                .map(|entry| (entry.state, entry.actionable_now, entry.maturity)),
            Some((ExecutorState::Operational, true, ExecutorMaturity::OperationalAdvisory))
        );
        assert_eq!(
            values
                .iter()
                .find(|entry| entry.executor_id == "numerical.evolve")
                .map(|entry| (entry.state, entry.actionable_now)),
            Some((ExecutorState::Operational, true))
        );
        assert_eq!(
            values
                .iter()
                .filter(|entry| entry.production_authority)
                .count(),
            1
        );
    }

    #[test]
    fn registry_marks_evidence_bound_plasticity_as_operational_advisory() {
        let values = executor_catalog().unwrap();
        for id in [
            "plasticity.bcm",
            "plasticity.eligibility",
            "plasticity.neuromodulation",
            "plasticity.routing",
            "plasticity.content",
            "plasticity.pi",
            "plasticity.elo",
            "coevolution.loop",
        ] {
            let entry = values
                .iter()
                .find(|entry| entry.executor_id == id)
                .unwrap_or_else(|| panic!("missing {id}"));
            assert_eq!(entry.state, ExecutorState::Operational, "id={id}");
            assert_eq!(
                entry.runtime_status,
                ExecutorRuntimeStatus::AdvisoryFromEvidence,
                "id={id}"
            );
            assert!(entry.actionable_now, "id={id}");
            assert!(!entry.production_authority, "id={id}");
        }
        assert_eq!(
            values
                .iter()
                .find(|entry| entry.executor_id == "consensus.builder")
                .map(|entry| entry.state),
            Some(ExecutorState::ArchitectureOnly)
        );
    }

    #[test]
    fn lifecycle_effect_requires_lifecycle_authority() {
        let bad = ExecutorDescriptor::new(ExecutorDescriptorDraft {
            executor_id: "bad.lifecycle",
            title: "Bad",
            module_path: "bad",
            owner: "bad",
            state: ExecutorState::Operational,
            authority: ExecutorAuthorityClass::Executor,
            effect_class: ExecutorEffectClass::ProductionLifecycle,
            surfaces: &[ExecutorSurface::InternalLibrary],
            accepted_needs: &["bad"],
            requires: &[],
            produces: &["bad"],
            operator_recipe_id: None,
            notes: "bad",
        });
        assert!(bad.is_err());
    }
}
