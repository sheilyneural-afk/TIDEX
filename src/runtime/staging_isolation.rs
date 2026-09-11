//! Staging-root admission. Candidate state must never share a root with activated production state.

use crate::foundation::authority::existing_directory_under_root;
use crate::foundation::error::{BrainError, BrainResult};
#[cfg(test)]
use crate::foundation::security::verify_internal_private_root;
use crate::foundation::security::verify_private_root;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagingRoots {
    staging_root: PathBuf,
    state_root: PathBuf,
    artifact_root: PathBuf,
}

impl StagingRoots {
    pub fn open(
        staging_root: &Path,
        state_root: &Path,
        artifact_root: &Path,
        production_root: &Path,
    ) -> BrainResult<Self> {
        let staging_root = verify_private_root(staging_root)?;
        Self::admit(staging_root, state_root, artifact_root, production_root)
    }

    #[cfg(test)]
    pub(crate) fn open_for_test(
        staging_root: &Path,
        state_root: &Path,
        artifact_root: &Path,
        production_root: &Path,
    ) -> BrainResult<Self> {
        let staging_root = verify_internal_private_root(staging_root)?;
        Self::admit(staging_root, state_root, artifact_root, production_root)
    }

    fn admit(
        staging_root: PathBuf,
        state_root: &Path,
        artifact_root: &Path,
        production_root: &Path,
    ) -> BrainResult<Self> {
        for (root, label) in [
            (state_root, "staging_state_root_invalid"),
            (artifact_root, "staging_artifact_root_invalid"),
        ] {
            let metadata =
                fs::symlink_metadata(root).map_err(|_| BrainError::Invalid(label.into()))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(BrainError::Integrity(label.into()));
            }
        }
        let state_root = state_root
            .canonicalize()
            .map_err(|_| BrainError::Invalid("staging_state_root_unavailable".into()))?;
        let artifact_root = artifact_root
            .canonicalize()
            .map_err(|_| BrainError::Invalid("staging_artifact_root_unavailable".into()))?;
        let production_root = production_root
            .canonicalize()
            .map_err(|_| BrainError::Invalid("production_root_unavailable".into()))?;
        if !state_root.starts_with(&staging_root)
            || !artifact_root.starts_with(&staging_root)
            || state_root.starts_with(&artifact_root)
            || artifact_root.starts_with(&state_root)
            || state_root.starts_with(&production_root)
            || artifact_root.starts_with(&production_root)
            || production_root.starts_with(&staging_root)
        {
            return Err(BrainError::Integrity("staging_roots_not_isolated".into()));
        }
        existing_directory_under_root(&staging_root, &state_root)?;
        existing_directory_under_root(&staging_root, &artifact_root)?;
        Ok(Self {
            staging_root,
            state_root,
            artifact_root,
        })
    }

    pub fn staging_root(&self) -> &Path {
        &self.staging_root
    }

    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    pub fn artifact_root(&self) -> &Path {
        &self.artifact_root
    }
}
