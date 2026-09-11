//! Fail-closed consensus reducer for candidate governance.
//!
//! Consensus is not production activation authority. It binds an exact proposal
//! payload, an explicit voter set, immutable votes, quorum and expiry. Invalid
//! state is an error, never silently converted to Pending or Approved.

use crate::cross_model::models::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalType {
    CapabilityTransfer,
    ModelUpdate,
    ParameterChange,
    Governance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vote {
    Approve,
    Reject,
    Abstain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusState {
    Pending,
    Approved,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConsensusProposal {
    pub schema: String,
    pub proposal_id: String,
    pub proposal_type: ProposalType,
    pub proposer: String,
    pub content_json: serde_json::Value,
    pub content_sha256: String,
    pub votes: BTreeMap<String, Vote>,
    pub voter_set_sha256: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConsensusBuilderConfig {
    pub approval_threshold: f64,
    pub quorum_threshold: f64,
    pub voting_timeout_seconds: u64,
    pub allow_single_voter: bool,
}

impl Default for ConsensusBuilderConfig {
    fn default() -> Self {
        Self {
            approval_threshold: 2.0 / 3.0,
            quorum_threshold: 0.5,
            voting_timeout_seconds: 3600,
            allow_single_voter: false,
        }
    }
}

impl ConsensusBuilderConfig {
    fn validate(&self) -> Result<(), String> {
        if !self.approval_threshold.is_finite()
            || !(0.5..=1.0).contains(&self.approval_threshold)
            || !self.quorum_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.quorum_threshold)
            || self.voting_timeout_seconds == 0
            || self.voting_timeout_seconds > 31_536_000
        {
            return Err("consensus_config_invalid".into());
        }
        Ok(())
    }
}

pub struct ConsensusBuilder {
    config: ConsensusBuilderConfig,
    proposals: HashMap<String, ConsensusProposal>,
    voters: BTreeSet<String>,
    proposal_counter: u64,
}

impl ConsensusBuilder {
    pub fn new(config: ConsensusBuilderConfig, voters: Vec<String>) -> Result<Self, String> {
        config.validate()?;
        let voters = voters
            .into_iter()
            .map(|v| v.trim().to_string())
            .collect::<BTreeSet<_>>();
        if voters.is_empty()
            || voters.iter().any(|v| v.is_empty())
            || (voters.len() == 1 && !config.allow_single_voter)
        {
            return Err("consensus_voter_set_invalid".into());
        }
        Ok(Self {
            config,
            proposals: HashMap::new(),
            voters,
            proposal_counter: 0,
        })
    }

    pub fn default(voters: Vec<String>) -> Result<Self, String> {
        Self::new(ConsensusBuilderConfig::default(), voters)
    }

    pub fn create_proposal(
        &mut self,
        proposal_type: ProposalType,
        proposer: &str,
        content_json: serde_json::Value,
    ) -> Result<ConsensusProposal, String> {
        if !self.voters.contains(proposer) {
            return Err("consensus_proposer_not_authorized".into());
        }
        let content_bytes = serde_json::to_vec(&content_json)
            .map_err(|e| format!("consensus_content_serialize:{e}"))?;
        if content_bytes.is_empty() || content_bytes.len() > 16 * 1024 * 1024 {
            return Err("consensus_content_invalid".into());
        }
        self.proposal_counter = self
            .proposal_counter
            .checked_add(1)
            .ok_or("consensus_counter_overflow")?;
        let content_sha256 = sha256_hex(&content_bytes);
        let voter_set_sha256 = sha256_hex(
            &serde_json::to_vec(&self.voters)
                .map_err(|e| format!("consensus_voters_serialize:{e}"))?,
        );
        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::seconds(self.config.voting_timeout_seconds as i64);
        let proposal_id = sha256_hex(
            format!(
                "{}\0{:?}\0{}\0{}\0{}",
                self.proposal_counter, proposal_type, proposer, content_sha256, voter_set_sha256
            )
            .as_bytes(),
        );
        let proposal = ConsensusProposal {
            schema: "cerebro.cross_model.consensus_proposal/v1".into(),
            proposal_id: proposal_id.clone(),
            proposal_type,
            proposer: proposer.into(),
            content_json,
            content_sha256,
            votes: BTreeMap::new(),
            voter_set_sha256,
            created_at: now.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
        };
        self.proposals.insert(proposal_id, proposal.clone());
        Ok(proposal)
    }

    pub fn cast_vote(&mut self, proposal_id: &str, voter: &str, vote: Vote) -> Result<(), String> {
        if !self.voters.contains(voter) {
            return Err("consensus_voter_not_authorized".into());
        }
        // Resolve expiry before mutable borrow.
        if self.get_consensus_state(proposal_id)? == ConsensusState::Expired {
            return Err("consensus_proposal_expired".into());
        }
        let proposal = self
            .proposals
            .get_mut(proposal_id)
            .ok_or("consensus_proposal_missing")?;
        if proposal.votes.contains_key(voter) {
            return Err("consensus_duplicate_vote".into());
        }
        proposal.votes.insert(voter.into(), vote);
        Ok(())
    }

    pub fn get_consensus_state(&self, proposal_id: &str) -> Result<ConsensusState, String> {
        let proposal = self
            .proposals
            .get(proposal_id)
            .ok_or("consensus_proposal_missing")?;
        if proposal.schema != "cerebro.cross_model.consensus_proposal/v1" {
            return Err("consensus_proposal_schema_invalid".into());
        }
        let expires = chrono::DateTime::parse_from_rfc3339(&proposal.expires_at)
            .map_err(|_| "consensus_expiry_invalid".to_string())?
            .with_timezone(&chrono::Utc);
        if chrono::Utc::now() > expires {
            return Ok(ConsensusState::Expired);
        }
        if proposal
            .votes
            .keys()
            .any(|voter| !self.voters.contains(voter))
        {
            return Err("consensus_vote_from_unknown_voter".into());
        }
        let voter_hash = sha256_hex(
            &serde_json::to_vec(&self.voters)
                .map_err(|e| format!("consensus_voters_serialize:{e}"))?,
        );
        if voter_hash != proposal.voter_set_sha256 {
            return Err("consensus_voter_set_changed_after_proposal".into());
        }
        if self.voters.len() == 1 {
            if !self.config.allow_single_voter {
                return Err("consensus_single_voter_forbidden".into());
            }
            return Ok(match proposal.votes.values().next() {
                Some(Vote::Approve) => ConsensusState::Approved,
                Some(Vote::Reject) => ConsensusState::Rejected,
                Some(Vote::Abstain) | None => ConsensusState::Pending,
            });
        }
        let participation = proposal.votes.len() as f64 / self.voters.len() as f64;
        if participation < self.config.quorum_threshold {
            return Ok(ConsensusState::Pending);
        }
        let decisive = proposal
            .votes
            .values()
            .filter(|vote| **vote != Vote::Abstain)
            .count();
        if decisive == 0 {
            return Ok(ConsensusState::Pending);
        }
        let approvals = proposal
            .votes
            .values()
            .filter(|vote| **vote == Vote::Approve)
            .count();
        let rejections = proposal
            .votes
            .values()
            .filter(|vote| **vote == Vote::Reject)
            .count();
        let approval_ratio = approvals as f64 / decisive as f64;
        let rejection_ratio = rejections as f64 / decisive as f64;
        if approval_ratio >= self.config.approval_threshold {
            Ok(ConsensusState::Approved)
        } else if rejection_ratio > 1.0 - self.config.approval_threshold {
            Ok(ConsensusState::Rejected)
        } else {
            Ok(ConsensusState::Pending)
        }
    }

    pub fn get_proposal(&self, proposal_id: &str) -> Option<&ConsensusProposal> {
        self.proposals.get(proposal_id)
    }

    pub fn proposals_with_state(
        &self,
        state: ConsensusState,
    ) -> Result<Vec<&ConsensusProposal>, String> {
        let mut rows = Vec::new();
        for proposal in self.proposals.values() {
            if self.get_consensus_state(&proposal.proposal_id)? == state {
                rows.push(proposal);
            }
        }
        rows.sort_by(|a, b| a.proposal_id.cmp(&b.proposal_id));
        Ok(rows)
    }

    pub fn get_pending_proposals(&self) -> Result<Vec<&ConsensusProposal>, String> {
        self.proposals_with_state(ConsensusState::Pending)
    }
    pub fn get_approved_proposals(&self) -> Result<Vec<&ConsensusProposal>, String> {
        self.proposals_with_state(ConsensusState::Approved)
    }

    pub fn add_voter(&mut self, _voter: String) -> Result<(), String> {
        Err("consensus_voter_set_is_immutable_after_initialization".into())
    }

    pub fn remove_voter(&mut self, _voter: &str) -> Result<(), String> {
        Err("consensus_voter_set_is_immutable_after_initialization".into())
    }

    pub fn get_voters(&self) -> Vec<&str> {
        self.voters.iter().map(String::as_str).collect()
    }

    pub fn clear_expired(&mut self) -> Result<usize, String> {
        let ids = self.proposals.keys().cloned().collect::<Vec<_>>();
        let expired = ids
            .into_iter()
            .filter_map(|id| match self.get_consensus_state(&id) {
                Ok(ConsensusState::Expired) => Some(Ok(id)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        for id in &expired {
            self.proposals.remove(id);
        }
        Ok(expired.len())
    }

    pub fn get_statistics(&self) -> Result<ConsensusStatistics, String> {
        let mut stats = ConsensusStatistics {
            total_proposals: self.proposals.len(),
            approved: 0,
            rejected: 0,
            pending: 0,
            expired: 0,
            total_voters: self.voters.len(),
        };
        for proposal in self.proposals.values() {
            match self.get_consensus_state(&proposal.proposal_id)? {
                ConsensusState::Approved => stats.approved += 1,
                ConsensusState::Rejected => stats.rejected += 1,
                ConsensusState::Pending => stats.pending += 1,
                ConsensusState::Expired => stats.expired += 1,
            }
        }
        Ok(stats)
    }

    pub fn reset(&mut self) {
        self.proposals.clear();
        self.proposal_counter = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConsensusStatistics {
    pub total_proposals: usize,
    pub approved: usize,
    pub rejected: usize,
    pub pending: usize,
    pub expired: usize,
    pub total_voters: usize,
}
