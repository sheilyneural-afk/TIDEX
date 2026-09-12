use crate::foundation::contracts::ConfounderValue;
use crate::foundation::digest::Sha256Digest;
use crate::foundation::error::{BrainError, BrainResult};
use crate::foundation::identity::SkillId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const EFFECT_95_Z: f64 = 1.959_963_984_540_054;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CounterfactualEvaluation {
    pub context_id: String,
    /// Statistical independence unit. Multiple tasks evaluated under the same
    /// randomization/seed share one group and must not be treated as replicates.
    #[serde(default)]
    pub independence_group: String,
    pub active_fields: Vec<SkillId>,
    pub utility: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldCausalCredit {
    pub skill_id: SkillId,
    pub matched_pairs: usize,
    pub independent_contexts: usize,
    pub mean_marginal_effect: f64,
    pub standard_error: f64,
    pub lower_confidence_bound: f64,
    pub positive_fraction: f64,
    /// `resolved` means the effect is statistically estimable from enough
    /// independent groups. It does NOT itself claim positive benefit.
    pub resolved: bool,
    pub beneficial: bool,
    #[serde(default)]
    pub shapley_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairInteractionCredit {
    pub left_skill_id: SkillId,
    pub right_skill_id: SkillId,
    pub matched_quads: usize,
    pub independent_contexts: usize,
    pub mean_interaction_effect: f64,
    pub standard_error: f64,
    pub resolved: bool,
}

impl PairInteractionCredit {
    /// Pessimistic 95% interaction effect used by governed routing. Positive
    /// synergy is admitted only when it survives its uncertainty margin;
    /// possible negative interference is deliberately retained.
    pub fn lower_confidence_bound(&self) -> BrainResult<f64> {
        if !self.resolved
            || !self.mean_interaction_effect.is_finite()
            || !self.standard_error.is_finite()
            || self.standard_error < 0.0
        {
            return Err(BrainError::Integrity("causal_pair_interaction_unresolved".into()));
        }
        let bound = self.mean_interaction_effect - EFFECT_95_Z * self.standard_error;
        if !bound.is_finite() {
            return Err(BrainError::Numerical("causal_pair_interaction_bound_non_finite".into()));
        }
        Ok(bound)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CausalCreditReport {
    pub schema: String,
    pub context_count: usize,
    pub independent_group_count: usize,
    pub field_count: usize,
    pub fields: Vec<FieldCausalCredit>,
    pub pair_interactions: Vec<PairInteractionCredit>,
    pub unresolved_fields: Vec<SkillId>,
}

/// Return the conservative, evidence-backed causal utility weight for every
/// runtime field in the caller's canonical order.  The lower confidence bound,
/// rather than the point estimate, is the only quantity that may prioritize
/// preservation inside a governed trust region.  Any missing, unresolved, or
/// non-beneficial field is an authority failure: composition must not silently
/// substitute a heuristic weight.
pub fn certified_causal_priority_weights(
    report: &CausalCreditReport,
    field_ids: &[SkillId],
) -> BrainResult<Vec<f64>> {
    if report.schema != "tidex.causal_credit/v3"
        || field_ids.is_empty()
        || report.field_count != field_ids.len()
        || report.independent_group_count < 3
        || !report.unresolved_fields.is_empty()
    {
        return Err(BrainError::Integrity("causal_credit_priority_report_unresolved".into()));
    }
    let expected = field_ids.iter().cloned().collect::<BTreeSet<_>>();
    if expected.len() != field_ids.len() {
        return Err(BrainError::Invalid("causal_credit_priority_field_identity_invalid".into()));
    }
    let mut by_id = BTreeMap::new();
    for field in &report.fields {
        if field.skill_id.trim().is_empty()
            || !field.resolved
            || !field.beneficial
            || field.matched_pairs == 0
            || field.independent_contexts < 3
            || !field.mean_marginal_effect.is_finite()
            || !field.standard_error.is_finite()
            || field.standard_error < 0.0
            || !field.lower_confidence_bound.is_finite()
            || field.lower_confidence_bound <= 0.0
            || !field.positive_fraction.is_finite()
            || !(0.0..=1.0).contains(&field.positive_fraction)
            || by_id
                .insert(field.skill_id.clone(), field.lower_confidence_bound)
                .is_some()
        {
            return Err(BrainError::Integrity("causal_credit_priority_field_unresolved".into()));
        }
    }
    if by_id.len() != expected.len()
        || by_id.keys().any(|id| !expected.contains(id))
        || report.pair_interactions.len() != expected.len() * expected.len().saturating_sub(1) / 2
    {
        return Err(BrainError::Integrity("causal_credit_priority_identity_mismatch".into()));
    }
    let mut pairs = BTreeSet::new();
    for pair in &report.pair_interactions {
        let (left, right) = if pair.left_skill_id < pair.right_skill_id {
            (&pair.left_skill_id, &pair.right_skill_id)
        } else {
            (&pair.right_skill_id, &pair.left_skill_id)
        };
        if left == right
            || !expected.contains(left)
            || !expected.contains(right)
            || !pair.resolved
            || pair.matched_quads == 0
            || pair.independent_contexts < 3
            || !pair.mean_interaction_effect.is_finite()
            || !pair.standard_error.is_finite()
            || pair.standard_error < 0.0
            || pair.lower_confidence_bound().is_err()
            || !pairs.insert((left.clone(), right.clone()))
        {
            return Err(BrainError::Integrity(
                "causal_credit_priority_interaction_unresolved".into(),
            ));
        }
    }
    field_ids
        .iter()
        .map(|field_id| {
            by_id
                .get(field_id)
                .copied()
                .ok_or_else(|| BrainError::Integrity("causal_credit_priority_field_missing".into()))
        })
        .collect::<BrainResult<Vec<_>>>()
}

fn canonical_set(fields: &[SkillId]) -> BTreeSet<SkillId> {
    fields.iter().cloned().collect()
}

fn mean_and_se(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, f64::INFINITY);
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    if values.len() < 2 {
        return (mean, f64::INFINITY);
    }
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    (mean, (variance / values.len() as f64).sqrt())
}

fn independent_means(
    context_values: &[(String, f64)],
    context_groups: &BTreeMap<String, String>,
) -> BrainResult<Vec<f64>> {
    let mut grouped = BTreeMap::<String, Vec<f64>>::new();
    for (context, value) in context_values {
        let group = context_groups
            .get(context)
            .ok_or_else(|| BrainError::Integrity("causal_credit_context_group_missing".into()))?;
        grouped.entry(group.clone()).or_default().push(*value);
    }
    Ok(grouped
        .into_values()
        .map(|values| values.iter().sum::<f64>() / values.len() as f64)
        .collect())
}

pub fn estimate_causal_credit(
    evaluations: &[CounterfactualEvaluation],
) -> BrainResult<CausalCreditReport> {
    if evaluations.is_empty() {
        return Err(BrainError::Invalid("causal_credit_empty".into()));
    }
    let mut contexts = BTreeSet::new();
    let mut independent_groups = BTreeSet::new();
    let mut context_groups = BTreeMap::<String, String>::new();
    let mut fields = BTreeSet::new();
    let mut table = BTreeMap::<(String, BTreeSet<SkillId>), f64>::new();
    for (index, evaluation) in evaluations.iter().enumerate() {
        if evaluation.context_id.trim().is_empty() || !evaluation.utility.is_finite() {
            return Err(BrainError::Invalid(format!("causal_credit_evaluation_invalid:{index}")));
        }
        let group = if evaluation.independence_group.trim().is_empty() {
            evaluation.context_id.clone()
        } else {
            evaluation.independence_group.clone()
        };
        if let Some(existing) = context_groups.insert(evaluation.context_id.clone(), group.clone())
        {
            if existing != group {
                return Err(BrainError::Invalid(
                    "causal_credit_context_independence_group_inconsistent".into(),
                ));
            }
        }
        let active = canonical_set(&evaluation.active_fields);
        if active.len() != evaluation.active_fields.len() {
            return Err(BrainError::Invalid(format!("causal_credit_duplicate_field:{index}")));
        }
        contexts.insert(evaluation.context_id.clone());
        independent_groups.insert(group);
        fields.extend(active.iter().cloned());
        let key = (evaluation.context_id.clone(), active);
        if table.insert(key, evaluation.utility).is_some() {
            return Err(BrainError::Invalid("causal_credit_duplicate_cell".into()));
        }
    }

    let field_ids = fields.iter().cloned().collect::<Vec<_>>();
    let shapley_map = compute_shapley_values(evaluations).unwrap_or_default();
    let mut field_reports = Vec::with_capacity(field_ids.len());
    let mut unresolved = Vec::new();
    for field in &field_ids {
        let mut raw_pair_count = 0usize;
        let mut context_effects = Vec::<(String, f64)>::new();
        for context in &contexts {
            let cells = table
                .iter()
                .filter(|((ctx, _), _)| ctx == context)
                .map(|((_, set), utility)| (set.clone(), *utility))
                .collect::<Vec<_>>();
            let mut local_effects = Vec::new();
            for (set, utility_without) in &cells {
                if set.contains(field) {
                    continue;
                }
                let mut with = set.clone();
                with.insert(field.clone());
                if let Some(utility_with) = table.get(&(context.clone(), with)) {
                    local_effects.push(*utility_with - *utility_without);
                    raw_pair_count += 1;
                }
            }
            if !local_effects.is_empty() {
                context_effects.push((
                    context.clone(),
                    local_effects.iter().sum::<f64>() / local_effects.len() as f64,
                ));
            }
        }
        let effects = independent_means(&context_effects, &context_groups)?;
        let (mean, se) = mean_and_se(&effects);
        let lower_confidence_bound = if se.is_finite() {
            mean - EFFECT_95_Z * se
        } else {
            f64::NEG_INFINITY
        };
        let positive_fraction = if effects.is_empty() {
            0.0
        } else {
            effects.iter().filter(|value| **value > 0.0).count() as f64 / effects.len() as f64
        };
        let resolved = effects.len() >= 3 && se.is_finite();
        let beneficial = resolved && lower_confidence_bound > 0.0;
        if !resolved {
            unresolved.push(field.clone());
        }
        let sv = shapley_map.get(field).copied();
        field_reports.push(FieldCausalCredit {
            skill_id: field.clone(),
            matched_pairs: raw_pair_count,
            independent_contexts: effects.len(),
            mean_marginal_effect: mean,
            standard_error: se,
            lower_confidence_bound,
            positive_fraction,
            resolved,
            beneficial,
            shapley_value: sv,
        });
    }

    let mut pair_interactions = Vec::new();
    for i in 0..field_ids.len() {
        for j in 0..i {
            let left = &field_ids[j];
            let right = &field_ids[i];
            let mut raw_quad_count = 0usize;
            let mut context_interactions = Vec::<(String, f64)>::new();
            for context in &contexts {
                let cells = table
                    .iter()
                    .filter(|((ctx, _), _)| ctx == context)
                    .map(|((_, set), utility)| (set.clone(), *utility))
                    .collect::<Vec<_>>();
                let mut local_interactions = Vec::new();
                for (base, u00) in &cells {
                    if base.contains(left) || base.contains(right) {
                        continue;
                    }
                    let mut only_left = base.clone();
                    only_left.insert(left.clone());
                    let mut only_right = base.clone();
                    only_right.insert(right.clone());
                    let mut both = only_left.clone();
                    both.insert(right.clone());
                    if let (Some(u10), Some(u01), Some(u11)) = (
                        table.get(&(context.clone(), only_left)),
                        table.get(&(context.clone(), only_right)),
                        table.get(&(context.clone(), both)),
                    ) {
                        local_interactions.push(*u11 - *u10 - *u01 + *u00);
                        raw_quad_count += 1;
                    }
                }
                if !local_interactions.is_empty() {
                    context_interactions.push((
                        context.clone(),
                        local_interactions.iter().sum::<f64>() / local_interactions.len() as f64,
                    ));
                }
            }
            let effects = independent_means(&context_interactions, &context_groups)?;
            let (mean, se) = mean_and_se(&effects);
            pair_interactions.push(PairInteractionCredit {
                left_skill_id: left.clone(),
                right_skill_id: right.clone(),
                matched_quads: raw_quad_count,
                independent_contexts: effects.len(),
                mean_interaction_effect: mean,
                standard_error: se,
                resolved: effects.len() >= 3 && se.is_finite(),
            });
        }
    }
    Ok(CausalCreditReport {
        schema: "tidex.causal_credit/v3".into(),
        context_count: contexts.len(),
        independent_group_count: independent_groups.len(),
        field_count: field_ids.len(),
        fields: field_reports,
        pair_interactions,
        unresolved_fields: unresolved,
    })
}

fn factorial(n: usize) -> f64 {
    (1..=n).map(|x| x as f64).product()
}

/// Computes formal N-player Shapley Values for each skill field across contexts:
/// \phi_i = \sum_{S \subseteq N \setminus \{i\}} \frac{|S|!(|N|-|S|-1)!}{|N|!} ( v(S \cup \{i\}) - v(S) )
pub fn compute_shapley_values(
    evaluations: &[CounterfactualEvaluation],
) -> BrainResult<BTreeMap<SkillId, f64>> {
    let mut all_skills = BTreeSet::new();
    let mut table = BTreeMap::<(String, BTreeSet<SkillId>), f64>::new();
    let mut contexts = BTreeSet::new();

    for eval in evaluations {
        let set = canonical_set(&eval.active_fields);
        all_skills.extend(set.iter().cloned());
        contexts.insert(eval.context_id.clone());
        table.insert((eval.context_id.clone(), set), eval.utility);
    }

    let skills: Vec<SkillId> = all_skills.into_iter().collect();
    let n = skills.len();
    if n == 0 {
        return Ok(BTreeMap::new());
    }

    let mut shapley_map = BTreeMap::new();

    for (i_idx, field) in skills.iter().enumerate() {
        let other_skills: Vec<SkillId> = skills
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i_idx)
            .map(|(_, s)| s.clone())
            .collect();
        let m = other_skills.len();
        let total_subsets = if m < 20 { 1usize << m } else { 1024 }; // Bound subset evaluation for high dimensions

        let mut total_shapley = 0.0;
        let mut total_weight = 0.0;

        for mask in 0..total_subsets {
            let mut subset = BTreeSet::new();
            for (bit, s) in other_skills.iter().enumerate().take(20) {
                if (mask & (1 << bit)) != 0 {
                    subset.insert(s.clone());
                }
            }

            let subset_len = subset.len();
            let mut subset_with_field = subset.clone();
            subset_with_field.insert(field.clone());

            let mut diffs = Vec::new();
            for ctx in &contexts {
                if let (Some(u_with), Some(u_without)) = (
                    table.get(&(ctx.clone(), subset_with_field.clone())),
                    table.get(&(ctx.clone(), subset.clone())),
                ) {
                    diffs.push(u_with - u_without);
                }
            }

            if !diffs.is_empty() {
                let avg_diff = diffs.iter().sum::<f64>() / diffs.len() as f64;
                let weight = (factorial(subset_len) * factorial(n - 1 - subset_len)) / factorial(n);
                total_shapley += weight * avg_diff;
                total_weight += weight;
            }
        }

        let final_val = if total_weight > 0.0 {
            total_shapley / total_weight
        } else {
            0.0
        };
        shapley_map.insert(field.clone(), final_val);
    }

    Ok(shapley_map)
}

/// Compute-matched evidence that prior acquisition of `source_skill_ids`
/// changes learning utility on `target_skill_id`. Baseline and facilitated
/// outcomes live in the same record so the pairing cannot be reconstructed
/// from unrelated runs. `matched_design_digest` binds the caller's immutable
/// compute/data/optimizer design; this reducer never treats a label as proof
/// that two experiments were actually matched.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DirectedFacilitationObservation {
    pub target_skill_id: SkillId,
    pub source_skill_ids: Vec<SkillId>,
    /// Environmental/runtime conditions under which this paired facilitation
    /// observation was measured. The schema is canonicalized and becomes part
    /// of the edge identity; evidence from different contexts is never pooled.
    #[serde(default)]
    pub context: Vec<ConfounderValue>,
    pub independence_group: String,
    pub baseline_utility: f64,
    pub facilitated_utility: f64,
    pub matched_design_digest: Sha256Digest,
    pub evidence_digest: Sha256Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DirectedFacilitationEdge {
    pub target_skill_id: SkillId,
    pub source_skill_ids: Vec<SkillId>,
    /// Canonical context/confounder values for which this edge is established.
    pub context: Vec<ConfounderValue>,
    pub independent_groups: usize,
    pub paired_observations: usize,
    pub mean_effect: f64,
    pub standard_error: f64,
    pub lower_confidence_bound: f64,
    pub upper_confidence_bound: f64,
    pub positive_fraction: f64,
    pub resolved: bool,
    pub beneficial: bool,
    pub interfering: bool,
    pub matched_design_digests: Vec<Sha256Digest>,
    pub evidence_digests: Vec<Sha256Digest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DirectedFacilitationReport {
    pub schema: String,
    pub edge_count: usize,
    pub edges: Vec<DirectedFacilitationEdge>,
}

fn canonical_context(context: &[ConfounderValue]) -> BrainResult<Vec<ConfounderValue>> {
    let mut canonical = context.to_vec();
    if canonical
        .iter()
        .any(|item| item.name.trim().is_empty() || !item.value.is_finite())
    {
        return Err(BrainError::Invalid("directed_facilitation_context_invalid".into()));
    }
    canonical.sort_by(|left, right| left.name.cmp(&right.name));
    if canonical
        .windows(2)
        .any(|pair| pair[0].name == pair[1].name)
    {
        return Err(BrainError::Invalid("directed_facilitation_context_duplicate".into()));
    }
    Ok(canonical)
}

fn context_key(context: &[ConfounderValue]) -> BrainResult<Vec<(String, u64)>> {
    Ok(canonical_context(context)?
        .into_iter()
        .map(|item| (item.name, item.value.to_bits()))
        .collect())
}

fn canonical_sources(sources: &[SkillId], target: &SkillId) -> BrainResult<Vec<SkillId>> {
    if sources.is_empty() || sources.iter().any(|source| source == target) {
        return Err(BrainError::Invalid("directed_facilitation_source_identity_invalid".into()));
    }
    let mut canonical = sources.to_vec();
    canonical.sort();
    canonical.dedup();
    if canonical.len() != sources.len() {
        return Err(BrainError::Invalid("directed_facilitation_source_identity_invalid".into()));
    }
    Ok(canonical)
}

/// Estimate a directed first- or higher-order capability taskonomy from
/// compute-matched paired outcomes. Direction is explicit: {A,C}->B and B->A
/// are different edges. Independent groups, not repeated rows, determine
/// statistical resolution. Negative upper confidence bound means certified
/// interference; positive lower confidence bound means certified facilitation.
pub fn estimate_directed_facilitation(
    observations: &[DirectedFacilitationObservation],
) -> BrainResult<DirectedFacilitationReport> {
    if observations.is_empty() {
        return Err(BrainError::Invalid("directed_facilitation_empty".into()));
    }
    type ContextKey = Vec<(String, u64)>;
    type RouteKey = (SkillId, Vec<SkillId>, ContextKey);
    let mut grouped =
        BTreeMap::<RouteKey, BTreeMap<String, Vec<(f64, Sha256Digest, Sha256Digest)>>>::new();
    let mut evidence_seen = BTreeSet::new();
    for observation in observations {
        if observation.independence_group.trim().is_empty()
            || !observation.baseline_utility.is_finite()
            || !observation.facilitated_utility.is_finite()
            || !evidence_seen.insert(observation.evidence_digest.clone())
        {
            return Err(BrainError::Invalid("directed_facilitation_observation_invalid".into()));
        }
        let sources =
            canonical_sources(&observation.source_skill_ids, &observation.target_skill_id)?;
        let context = context_key(&observation.context)?;
        grouped
            .entry((observation.target_skill_id.clone(), sources, context))
            .or_default()
            .entry(observation.independence_group.clone())
            .or_default()
            .push((
                observation.facilitated_utility - observation.baseline_utility,
                observation.matched_design_digest.clone(),
                observation.evidence_digest.clone(),
            ));
    }

    let mut edges = Vec::with_capacity(grouped.len());
    for ((target_skill_id, source_skill_ids, context_key), groups) in grouped {
        let context = context_key
            .into_iter()
            .map(|(name, bits)| ConfounderValue {
                name,
                value: f64::from_bits(bits),
            })
            .collect::<Vec<_>>();
        let mut independent_effects = Vec::with_capacity(groups.len());
        let mut design_digests = BTreeSet::new();
        let mut evidence_digests = BTreeSet::new();
        let mut paired_observations = 0usize;
        for rows in groups.values() {
            if rows.is_empty() {
                return Err(BrainError::Integrity("directed_facilitation_group_empty".into()));
            }
            // A statistical group must use one compute-matched design. Mixing
            // designs inside a claimed replicate group fails closed.
            let designs = rows
                .iter()
                .map(|(_, design, _)| design)
                .collect::<BTreeSet<_>>();
            if designs.len() != 1 {
                return Err(BrainError::Integrity(
                    "directed_facilitation_group_design_mismatch".into(),
                ));
            }
            design_digests.extend(rows.iter().map(|(_, design, _)| design.clone()));
            evidence_digests.extend(rows.iter().map(|(_, _, evidence)| evidence.clone()));
            paired_observations += rows.len();
            independent_effects
                .push(rows.iter().map(|(effect, _, _)| *effect).sum::<f64>() / rows.len() as f64);
        }
        let (mean_effect, standard_error) = mean_and_se(&independent_effects);
        let resolved = independent_effects.len() >= 3 && standard_error.is_finite();
        let (lower_confidence_bound, upper_confidence_bound) = if resolved {
            (
                mean_effect - EFFECT_95_Z * standard_error,
                mean_effect + EFFECT_95_Z * standard_error,
            )
        } else {
            (f64::NEG_INFINITY, f64::INFINITY)
        };
        let positive_fraction = independent_effects
            .iter()
            .filter(|effect| **effect > 0.0)
            .count() as f64
            / independent_effects.len() as f64;
        edges.push(DirectedFacilitationEdge {
            target_skill_id,
            source_skill_ids,
            context,
            independent_groups: independent_effects.len(),
            paired_observations,
            mean_effect,
            standard_error,
            lower_confidence_bound,
            upper_confidence_bound,
            positive_fraction,
            resolved,
            beneficial: resolved && lower_confidence_bound > 0.0,
            interfering: resolved && upper_confidence_bound < 0.0,
            matched_design_digests: design_digests.into_iter().collect(),
            evidence_digests: evidence_digests.into_iter().collect(),
        });
    }
    edges.sort_by(|left, right| {
        left.target_skill_id
            .cmp(&right.target_skill_id)
            .then_with(|| left.source_skill_ids.cmp(&right.source_skill_ids))
            .then_with(|| {
                context_key(&left.context)
                    .unwrap_or_default()
                    .cmp(&context_key(&right.context).unwrap_or_default())
            })
    });
    Ok(DirectedFacilitationReport {
        schema: "tidex.directed_facilitation/v1".into(),
        edge_count: edges.len(),
        edges,
    })
}

/// Project only resolved first-order edges into the canonical field order for
/// a target. This is deliberately a drive vector, not execution authority: it
/// can feed `CognitiveFieldDrive.evidence` while existing routing, risk and
/// promotion gates remain authoritative. Unresolved edges fail closed rather
/// than being silently treated as neutral evidence.
pub fn directed_facilitation_drive(
    report: &DirectedFacilitationReport,
    target_skill_id: &SkillId,
    context: &[ConfounderValue],
    field_ids: &[SkillId],
) -> BrainResult<Vec<f64>> {
    let requested_context = context_key(context)?;
    if report.schema != "tidex.directed_facilitation/v1"
        || report.edge_count != report.edges.len()
        || field_ids.is_empty()
        || field_ids.iter().collect::<BTreeSet<_>>().len() != field_ids.len()
    {
        return Err(BrainError::Integrity("directed_facilitation_report_invalid".into()));
    }
    let mut by_source = BTreeMap::<&SkillId, &DirectedFacilitationEdge>::new();
    for edge in &report.edges {
        if &edge.target_skill_id != target_skill_id
            || edge.source_skill_ids.len() != 1
            || context_key(&edge.context)? != requested_context
        {
            continue;
        }
        if !edge.resolved
            || !edge.mean_effect.is_finite()
            || !edge.standard_error.is_finite()
            || !edge.lower_confidence_bound.is_finite()
            || !edge.upper_confidence_bound.is_finite()
            || by_source.insert(&edge.source_skill_ids[0], edge).is_some()
        {
            return Err(BrainError::Integrity("directed_facilitation_edge_unresolved".into()));
        }
    }
    field_ids
        .iter()
        .map(|field_id| {
            by_source
                .get(field_id)
                .map(|edge| {
                    if edge.beneficial {
                        edge.lower_confidence_bound
                    } else if edge.interfering {
                        edge.upper_confidence_bound
                    } else {
                        0.0
                    }
                })
                .ok_or_else(|| BrainError::Integrity("directed_facilitation_edge_missing".into()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn causal_credit_recovers_main_and_interaction_effects() {
        let mut evaluations = Vec::new();
        for context in ["c1", "c2", "c3"] {
            for mask in 0..4 {
                let mut active = Vec::new();
                if mask & 1 != 0 {
                    active.push(SkillId::parse("a").unwrap());
                }
                if mask & 2 != 0 {
                    active.push(SkillId::parse("b").unwrap());
                }
                let a = if mask & 1 != 0 { 2.0 } else { 0.0 };
                let b = if mask & 2 != 0 { 1.0 } else { 0.0 };
                let interaction = if mask == 3 { 0.5 } else { 0.0 };
                evaluations.push(CounterfactualEvaluation {
                    context_id: context.into(),
                    independence_group: context.into(),
                    active_fields: active,
                    utility: a + b + interaction,
                });
            }
        }
        let report = estimate_causal_credit(&evaluations).unwrap();
        assert_eq!(report.independent_group_count, 3);
        assert!(report.unresolved_fields.is_empty());
        let a = report
            .fields
            .iter()
            .find(|field| field.skill_id == "a")
            .unwrap();
        let b = report
            .fields
            .iter()
            .find(|field| field.skill_id == "b")
            .unwrap();
        assert!((a.mean_marginal_effect - 2.25).abs() < 1e-12);
        assert!((b.mean_marginal_effect - 1.25).abs() < 1e-12);
        let pair = &report.pair_interactions[0];
        assert!((pair.mean_interaction_effect - 0.5).abs() < 1e-12);
    }

    #[test]
    fn tasks_under_one_randomization_count_as_one_independent_group() {
        let mut evaluations = Vec::new();
        for seed in [1, 2, 3] {
            for task in ["a", "b"] {
                for mask in 0..2 {
                    evaluations.push(CounterfactualEvaluation {
                        context_id: format!("seed-{seed}:{task}"),
                        independence_group: format!("seed-{seed}"),
                        active_fields: if mask == 1 {
                            vec![SkillId::parse("skill").unwrap()]
                        } else {
                            vec![]
                        },
                        utility: if mask == 1 { 1.0 } else { 0.0 },
                    });
                }
            }
        }
        let report = estimate_causal_credit(&evaluations).unwrap();
        assert_eq!(report.context_count, 6);
        assert_eq!(report.independent_group_count, 3);
        assert_eq!(report.fields[0].independent_contexts, 3);
        assert!(report.fields[0].beneficial);
    }

    #[test]
    fn shapley_values_computed_correctly() {
        let evaluations = vec![
            CounterfactualEvaluation {
                context_id: "c1".into(),
                independence_group: "c1".into(),
                active_fields: vec![],
                utility: 0.0,
            },
            CounterfactualEvaluation {
                context_id: "c1".into(),
                independence_group: "c1".into(),
                active_fields: vec![SkillId::parse("a").unwrap()],
                utility: 2.0,
            },
            CounterfactualEvaluation {
                context_id: "c1".into(),
                independence_group: "c1".into(),
                active_fields: vec![SkillId::parse("b").unwrap()],
                utility: 1.0,
            },
            CounterfactualEvaluation {
                context_id: "c1".into(),
                independence_group: "c1".into(),
                active_fields: vec![SkillId::parse("a").unwrap(), SkillId::parse("b").unwrap()],
                utility: 3.5,
            },
        ];
        let shapley = compute_shapley_values(&evaluations).unwrap();
        // \phi_a = 0.5 * (2.0 - 0.0) + 0.5 * (3.5 - 1.0) = 1.0 + 1.25 = 2.25
        // \phi_b = 0.5 * (1.0 - 0.0) + 0.5 * (3.5 - 2.0) = 0.5 + 0.75 = 1.25
        assert!((shapley.get("a").unwrap() - 2.25).abs() < 1e-10);
        assert!((shapley.get("b").unwrap() - 1.25).abs() < 1e-10);
    }

    fn facilitation_observation(
        source: &[&str],
        target: &str,
        group: &str,
        baseline: f64,
        facilitated: f64,
        nonce: char,
    ) -> DirectedFacilitationObservation {
        DirectedFacilitationObservation {
            target_skill_id: SkillId::parse(target).unwrap(),
            source_skill_ids: source
                .iter()
                .map(|id| SkillId::parse(*id).unwrap())
                .collect(),
            context: Vec::new(),
            independence_group: group.into(),
            baseline_utility: baseline,
            facilitated_utility: facilitated,
            matched_design_digest: Sha256Digest::parse("a".repeat(64)).unwrap(),
            evidence_digest: Sha256Digest::parse(nonce.to_string().repeat(64)).unwrap(),
        }
    }

    #[test]
    fn directed_facilitation_is_asymmetric_and_preserves_interference() {
        let observations = vec![
            facilitation_observation(&["a"], "b", "g1", 0.0, 1.0, '1'),
            facilitation_observation(&["a"], "b", "g2", 0.0, 1.1, '2'),
            facilitation_observation(&["a"], "b", "g3", 0.0, 0.9, '3'),
            facilitation_observation(&["b"], "a", "g1", 0.0, -0.8, '4'),
            facilitation_observation(&["b"], "a", "g2", 0.0, -0.9, '5'),
            facilitation_observation(&["b"], "a", "g3", 0.0, -0.7, '6'),
        ];
        let report = estimate_directed_facilitation(&observations).unwrap();
        assert_eq!(report.edge_count, 2);
        let a_to_b = report
            .edges
            .iter()
            .find(|edge| edge.target_skill_id == "b")
            .unwrap();
        let b_to_a = report
            .edges
            .iter()
            .find(|edge| edge.target_skill_id == "a")
            .unwrap();
        assert!(a_to_b.beneficial);
        assert!(!a_to_b.interfering);
        assert!(b_to_a.interfering);
        assert!(!b_to_a.beneficial);
    }

    #[test]
    fn higher_order_route_is_distinct_and_first_order_drive_is_conservative() {
        let observations = vec![
            facilitation_observation(&["a"], "target", "g1", 0.0, 1.0, '1'),
            facilitation_observation(&["a"], "target", "g2", 0.0, 1.0, '2'),
            facilitation_observation(&["a"], "target", "g3", 0.0, 1.0, '3'),
            facilitation_observation(&["c"], "target", "g1", 0.0, -1.0, '4'),
            facilitation_observation(&["c"], "target", "g2", 0.0, -1.0, '5'),
            facilitation_observation(&["c"], "target", "g3", 0.0, -1.0, '6'),
            facilitation_observation(&["a", "c"], "target", "g1", 0.0, 2.0, '7'),
            facilitation_observation(&["a", "c"], "target", "g2", 0.0, 2.0, '8'),
            facilitation_observation(&["a", "c"], "target", "g3", 0.0, 2.0, '9'),
        ];
        let report = estimate_directed_facilitation(&observations).unwrap();
        assert_eq!(report.edge_count, 3);
        assert!(report
            .edges
            .iter()
            .any(|edge| edge.source_skill_ids.len() == 2));
        let drive = directed_facilitation_drive(
            &report,
            &SkillId::parse("target").unwrap(),
            &[],
            &[SkillId::parse("a").unwrap(), SkillId::parse("c").unwrap()],
        )
        .unwrap();
        assert!(drive[0] > 0.0);
        assert!(drive[1] < 0.0);
    }

    #[test]
    fn directed_facilitation_fails_closed_on_unresolved_or_mismatched_design() {
        let unresolved = vec![
            facilitation_observation(&["a"], "b", "g1", 0.0, 1.0, '1'),
            facilitation_observation(&["a"], "b", "g2", 0.0, 1.0, '2'),
        ];
        let report = estimate_directed_facilitation(&unresolved).unwrap();
        assert!(directed_facilitation_drive(
            &report,
            &SkillId::parse("b").unwrap(),
            &[],
            &[SkillId::parse("a").unwrap()],
        )
        .is_err());

        let mut mismatched = vec![
            facilitation_observation(&["a"], "b", "g1", 0.0, 1.0, '3'),
            facilitation_observation(&["a"], "b", "g1", 0.0, 1.0, '4'),
        ];
        mismatched[1].matched_design_digest = Sha256Digest::parse("b".repeat(64)).unwrap();
        assert!(estimate_directed_facilitation(&mismatched).is_err());
    }

    #[test]
    fn context_conditioning_prevents_false_global_facilitation() {
        let hot = vec![ConfounderValue {
            name: "receiver_temperature".into(),
            value: 1.0,
        }];
        let cold = vec![ConfounderValue {
            name: "receiver_temperature".into(),
            value: 0.0,
        }];
        let mut observations = Vec::new();
        for (index, group) in ["g1", "g2", "g3"].into_iter().enumerate() {
            let mut positive = facilitation_observation(
                &["a"],
                "b",
                group,
                0.0,
                1.0,
                char::from(b'1' + index as u8),
            );
            positive.context = hot.clone();
            observations.push(positive);
            let mut negative = facilitation_observation(
                &["a"],
                "b",
                group,
                0.0,
                -1.0,
                char::from(b'4' + index as u8),
            );
            negative.context = cold.clone();
            observations.push(negative);
        }
        let report = estimate_directed_facilitation(&observations).unwrap();
        assert_eq!(report.edge_count, 2);
        let field_ids = [SkillId::parse("a").unwrap()];
        let hot_drive =
            directed_facilitation_drive(&report, &SkillId::parse("b").unwrap(), &hot, &field_ids)
                .unwrap();
        let cold_drive =
            directed_facilitation_drive(&report, &SkillId::parse("b").unwrap(), &cold, &field_ids)
                .unwrap();
        assert!(hot_drive[0] > 0.0);
        assert!(cold_drive[0] < 0.0);
        assert!(directed_facilitation_drive(
            &report,
            &SkillId::parse("b").unwrap(),
            &[ConfounderValue {
                name: "receiver_temperature".into(),
                value: 2.0
            }],
            &field_ids,
        )
        .is_err());
    }
}
