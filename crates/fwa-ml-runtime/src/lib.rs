mod types;
pub use types::{ModelExplanation, ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer};

pub(crate) mod verify;

pub mod scorer_artifact;
pub use scorer_artifact::ArtifactModelScorer;

pub mod scorer_manifest;
pub use scorer_manifest::ServingManifestModelScorer;

pub mod scorer_http;
pub use scorer_http::HttpModelScorer;

pub mod scorer_heuristic;
pub use scorer_heuristic::HeuristicModelScorer;

#[cfg(test)]
mod tests;
