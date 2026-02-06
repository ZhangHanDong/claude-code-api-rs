//! Multi-factor relevance scoring for memory retrieval.
//!
//! This module implements the scoring algorithm that combines:
//! - Semantic similarity (from Meilisearch)
//! - Working directory matching
//! - File overlap (Jaccard index)
//! - Recency decay (exponential)

use std::collections::HashSet;
use std::path::Path;

/// Configuration for relevance scoring weights.
#[derive(Debug, Clone)]
pub struct RelevanceConfig {
    /// Weight for semantic similarity score (0.0-1.0)
    pub semantic_weight: f64,

    /// Weight for working directory match score (0.0-1.0)
    pub cwd_weight: f64,

    /// Weight for file overlap score (0.0-1.0)
    pub files_weight: f64,

    /// Weight for recency score (0.0-1.0)
    pub recency_weight: f64,

    /// Decay rate for recency (hours for score to drop to ~37%)
    pub recency_half_life_hours: f64,
}

impl Default for RelevanceConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.4,
            cwd_weight: 0.3,
            files_weight: 0.2,
            recency_weight: 0.1,
            recency_half_life_hours: 24.0,
        }
    }
}

impl RelevanceConfig {
    /// Creates a config with custom weights.
    ///
    /// # Panics
    /// Panics if weights don't sum to approximately 1.0 (within 0.01 tolerance).
    pub fn with_weights(semantic: f64, cwd: f64, files: f64, recency: f64) -> Self {
        let sum = semantic + cwd + files + recency;
        assert!(
            (sum - 1.0).abs() < 0.01,
            "Weights must sum to 1.0, got {sum}"
        );

        Self {
            semantic_weight: semantic,
            cwd_weight: cwd,
            files_weight: files,
            recency_weight: recency,
            ..Default::default()
        }
    }

    /// Returns the total of all weights (should be 1.0).
    pub fn total_weight(&self) -> f64 {
        self.semantic_weight + self.cwd_weight + self.files_weight + self.recency_weight
    }
}

/// Individual score components and total relevance.
#[derive(Debug, Clone, PartialEq)]
pub struct RelevanceScore {
    /// Semantic similarity score (0.0-1.0)
    pub semantic: f64,

    /// Working directory match score (0.0-1.0)
    pub cwd_match: f64,

    /// File overlap score using Jaccard index (0.0-1.0)
    pub files_overlap: f64,

    /// Recency score with exponential decay (0.0-1.0)
    pub recency: f64,

    /// Combined weighted total (0.0-1.0)
    pub total: f64,
}

impl RelevanceScore {
    /// Creates a new RelevanceScore with individual components.
    ///
    /// The total is computed using the provided config weights.
    pub fn new(
        semantic: f64,
        cwd_match: f64,
        files_overlap: f64,
        recency: f64,
        config: &RelevanceConfig,
    ) -> Self {
        let total = semantic * config.semantic_weight
            + cwd_match * config.cwd_weight
            + files_overlap * config.files_weight
            + recency * config.recency_weight;

        Self {
            semantic,
            cwd_match,
            files_overlap,
            recency,
            total,
        }
    }

    /// Creates a zero score (no relevance).
    pub fn zero() -> Self {
        Self {
            semantic: 0.0,
            cwd_match: 0.0,
            files_overlap: 0.0,
            recency: 0.0,
            total: 0.0,
        }
    }
}

/// Computes relevance scores for memory retrieval.
#[derive(Debug, Clone)]
pub struct RelevanceScorer {
    config: RelevanceConfig,
}

impl Default for RelevanceScorer {
    fn default() -> Self {
        Self::new(RelevanceConfig::default())
    }
}

impl RelevanceScorer {
    /// Creates a new RelevanceScorer with the given configuration.
    pub fn new(config: RelevanceConfig) -> Self {
        Self { config }
    }

    /// Returns the configuration.
    pub fn config(&self) -> &RelevanceConfig {
        &self.config
    }

    /// Normalizes a Meilisearch score to 0.0-1.0 range.
    ///
    /// Meilisearch returns scores that can vary widely. We use a sigmoid-like
    /// normalization to compress the range.
    pub fn semantic_score(&self, meilisearch_score: f64) -> f64 {
        // Meilisearch scores are typically 0-1 for BM25-like scoring
        // but can exceed 1.0. We clamp and normalize.
        (meilisearch_score.min(2.0) / 2.0).clamp(0.0, 1.0)
    }

    /// Computes the working directory match score.
    ///
    /// - 1.0 if directories are identical
    /// - 0.5 if one is a parent/child of the other
    /// - 0.25 if they share a common ancestor (up to 3 levels)
    /// - 0.0 otherwise
    pub fn cwd_match_score(&self, current_cwd: Option<&str>, stored_cwd: Option<&str>) -> f64 {
        match (current_cwd, stored_cwd) {
            (Some(current), Some(stored)) => {
                let current = Path::new(current);
                let stored = Path::new(stored);

                // Exact match
                if current == stored {
                    return 1.0;
                }

                // Parent/child relationship
                if current.starts_with(stored) || stored.starts_with(current) {
                    return 0.5;
                }

                // Common ancestor check (up to 3 levels)
                let common = self.common_ancestor(current, stored);
                if let Some(ancestor) = common {
                    let ancestor_depth = ancestor.components().count();
                    if ancestor_depth >= 2 {
                        // More shared path = higher score
                        return 0.25 * (ancestor_depth as f64 / 5.0).min(1.0);
                    }
                }

                0.0
            },
            // If either is missing, neutral score
            (None, _) | (_, None) => 0.0,
        }
    }

    /// Finds the common ancestor path of two paths.
    fn common_ancestor<'a>(&self, a: &'a Path, b: &Path) -> Option<&'a Path> {
        let mut common_len = 0;

        for (i, (comp_a, comp_b)) in a.components().zip(b.components()).enumerate() {
            if comp_a == comp_b {
                common_len = i + 1;
            } else {
                break;
            }
        }

        if common_len == 0 {
            return None;
        }

        // Return a reference to the input path up to the common length
        a.ancestors().find(|p| p.components().count() == common_len)
    }

    /// Computes file overlap using the Jaccard index.
    ///
    /// Jaccard index = |A ∩ B| / |A ∪ B|
    ///
    /// Returns 0.0 if either set is empty.
    pub fn files_overlap_score(&self, current_files: &[String], stored_files: &[String]) -> f64 {
        if current_files.is_empty() || stored_files.is_empty() {
            return 0.0;
        }

        let current_set: HashSet<&str> = current_files.iter().map(|s| s.as_str()).collect();
        let stored_set: HashSet<&str> = stored_files.iter().map(|s| s.as_str()).collect();

        let intersection_size = current_set.intersection(&stored_set).count();
        let union_size = current_set.union(&stored_set).count();

        if union_size == 0 {
            return 0.0;
        }

        intersection_size as f64 / union_size as f64
    }

    /// Computes recency score with exponential decay.
    ///
    /// Formula: e^(-age_hours / half_life_hours)
    ///
    /// - 1h ago: ~0.96
    /// - 24h ago: ~0.37 (with default half_life of 24h)
    /// - 72h ago: ~0.05
    pub fn recency_score(&self, age_hours: f64) -> f64 {
        if age_hours < 0.0 {
            return 1.0; // Future messages get full score
        }

        (-age_hours / self.config.recency_half_life_hours).exp()
    }

    /// Computes recency score from timestamps.
    ///
    /// # Arguments
    /// * `stored_timestamp` - Unix timestamp of the stored message
    /// * `current_timestamp` - Current Unix timestamp
    pub fn recency_score_from_timestamps(
        &self,
        stored_timestamp: i64,
        current_timestamp: i64,
    ) -> f64 {
        let age_seconds = (current_timestamp - stored_timestamp).max(0) as f64;
        let age_hours = age_seconds / 3600.0;
        self.recency_score(age_hours)
    }

    /// Computes the full relevance score for a stored message.
    ///
    /// # Arguments
    /// * `meilisearch_score` - The raw score from Meilisearch
    /// * `current_cwd` - Current working directory
    /// * `stored_cwd` - Working directory when the message was stored
    /// * `current_files` - Files in the current context
    /// * `stored_files` - Files touched in the stored message
    /// * `age_hours` - Age of the message in hours
    pub fn compute_score(
        &self,
        meilisearch_score: f64,
        current_cwd: Option<&str>,
        stored_cwd: Option<&str>,
        current_files: &[String],
        stored_files: &[String],
        age_hours: f64,
    ) -> RelevanceScore {
        let semantic = self.semantic_score(meilisearch_score);
        let cwd_match = self.cwd_match_score(current_cwd, stored_cwd);
        let files_overlap = self.files_overlap_score(current_files, stored_files);
        let recency = self.recency_score(age_hours);

        RelevanceScore::new(semantic, cwd_match, files_overlap, recency, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relevance_config_default() {
        let config = RelevanceConfig::default();

        assert!((config.total_weight() - 1.0).abs() < 0.001);
        assert_eq!(config.semantic_weight, 0.4);
        assert_eq!(config.cwd_weight, 0.3);
        assert_eq!(config.files_weight, 0.2);
        assert_eq!(config.recency_weight, 0.1);
    }

    #[test]
    fn test_relevance_config_custom_weights() {
        let config = RelevanceConfig::with_weights(0.5, 0.2, 0.2, 0.1);

        assert_eq!(config.semantic_weight, 0.5);
        assert_eq!(config.cwd_weight, 0.2);
    }

    #[test]
    #[should_panic(expected = "Weights must sum to 1.0")]
    fn test_relevance_config_invalid_weights() {
        RelevanceConfig::with_weights(0.5, 0.5, 0.5, 0.5); // Sum = 2.0
    }

    #[test]
    fn test_semantic_score_normalization() {
        let scorer = RelevanceScorer::default();

        assert_eq!(scorer.semantic_score(0.0), 0.0);
        assert_eq!(scorer.semantic_score(1.0), 0.5);
        assert_eq!(scorer.semantic_score(2.0), 1.0);
        assert_eq!(scorer.semantic_score(3.0), 1.0); // Clamped
        assert_eq!(scorer.semantic_score(-1.0), 0.0); // Negative clamped
    }

    #[test]
    fn test_cwd_match_exact() {
        let scorer = RelevanceScorer::default();

        let score = scorer.cwd_match_score(Some("/projects/my-app"), Some("/projects/my-app"));

        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_cwd_match_parent_child() {
        let scorer = RelevanceScorer::default();

        // Current is child of stored
        let score1 = scorer.cwd_match_score(Some("/projects/my-app/src"), Some("/projects/my-app"));
        assert_eq!(score1, 0.5);

        // Current is parent of stored
        let score2 = scorer.cwd_match_score(
            Some("/projects/my-app"),
            Some("/projects/my-app/src/components"),
        );
        assert_eq!(score2, 0.5);
    }

    #[test]
    fn test_cwd_match_common_ancestor() {
        let scorer = RelevanceScorer::default();

        let score = scorer.cwd_match_score(
            Some("/projects/my-app/frontend"),
            Some("/projects/my-app/backend"),
        );

        // Should have some score due to common ancestor /projects/my-app
        assert!(score > 0.0);
        assert!(score < 0.5);
    }

    #[test]
    fn test_cwd_match_no_relation() {
        let scorer = RelevanceScorer::default();

        let score =
            scorer.cwd_match_score(Some("/home/user/project-a"), Some("/var/www/project-b"));

        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_cwd_match_none() {
        let scorer = RelevanceScorer::default();

        assert_eq!(scorer.cwd_match_score(None, Some("/projects")), 0.0);
        assert_eq!(scorer.cwd_match_score(Some("/projects"), None), 0.0);
        assert_eq!(scorer.cwd_match_score(None, None), 0.0);
    }

    #[test]
    fn test_files_overlap_jaccard() {
        let scorer = RelevanceScorer::default();

        // 2 files in common out of 4 total unique files
        // A = {f1, f2, f3}, B = {f2, f3, f4}
        // Intersection = {f2, f3} = 2
        // Union = {f1, f2, f3, f4} = 4
        // Jaccard = 2/4 = 0.5
        let current = vec![
            "/f1.rs".to_string(),
            "/f2.rs".to_string(),
            "/f3.rs".to_string(),
        ];
        let stored = vec![
            "/f2.rs".to_string(),
            "/f3.rs".to_string(),
            "/f4.rs".to_string(),
        ];

        let score = scorer.files_overlap_score(&current, &stored);

        assert_eq!(score, 0.5);
    }

    #[test]
    fn test_files_overlap_identical() {
        let scorer = RelevanceScorer::default();

        let files = vec!["/a.rs".to_string(), "/b.rs".to_string()];

        let score = scorer.files_overlap_score(&files, &files);

        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_files_overlap_no_common() {
        let scorer = RelevanceScorer::default();

        let current = vec!["/a.rs".to_string()];
        let stored = vec!["/b.rs".to_string()];

        let score = scorer.files_overlap_score(&current, &stored);

        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_files_overlap_empty() {
        let scorer = RelevanceScorer::default();

        assert_eq!(scorer.files_overlap_score(&[], &["/a.rs".to_string()]), 0.0);
        assert_eq!(scorer.files_overlap_score(&["/a.rs".to_string()], &[]), 0.0);
        assert_eq!(scorer.files_overlap_score(&[], &[]), 0.0);
    }

    #[test]
    fn test_recency_score_decay() {
        let scorer = RelevanceScorer::default();

        // 0 hours = 1.0
        let score_0h = scorer.recency_score(0.0);
        assert!((score_0h - 1.0).abs() < 0.001);

        // 1 hour ≈ 0.959 (e^(-1/24))
        let score_1h = scorer.recency_score(1.0);
        assert!((score_1h - 0.959).abs() < 0.01);

        // 24 hours ≈ 0.368 (e^(-1))
        let score_24h = scorer.recency_score(24.0);
        assert!((score_24h - 0.368).abs() < 0.01);

        // 72 hours ≈ 0.050 (e^(-3))
        let score_72h = scorer.recency_score(72.0);
        assert!((score_72h - 0.050).abs() < 0.01);
    }

    #[test]
    fn test_recency_score_future() {
        let scorer = RelevanceScorer::default();

        // Negative age (future) should return 1.0
        assert_eq!(scorer.recency_score(-1.0), 1.0);
    }

    #[test]
    fn test_recency_score_from_timestamps() {
        let scorer = RelevanceScorer::default();

        let stored = 1700000000_i64;
        let current = stored + 3600; // 1 hour later

        let score = scorer.recency_score_from_timestamps(stored, current);

        // Should be approximately e^(-1/24) ≈ 0.959
        assert!((score - 0.959).abs() < 0.01);
    }

    #[test]
    fn test_compute_score_combined() {
        let scorer = RelevanceScorer::default();

        let score = scorer.compute_score(
            1.5, // meilisearch_score -> normalized to 0.75
            Some("/projects/app"),
            Some("/projects/app"), // exact match -> 1.0
            &["/src/main.rs".to_string()],
            &["/src/main.rs".to_string(), "/src/lib.rs".to_string()], // 1/2 = 0.5
            1.0,                                                      // 1 hour ago -> ~0.959
        );

        // semantic: 0.75 * 0.4 = 0.3
        // cwd: 1.0 * 0.3 = 0.3
        // files: 0.5 * 0.2 = 0.1
        // recency: 0.959 * 0.1 ≈ 0.096
        // total ≈ 0.796

        assert!((score.semantic - 0.75).abs() < 0.01);
        assert_eq!(score.cwd_match, 1.0);
        assert_eq!(score.files_overlap, 0.5);
        assert!((score.recency - 0.959).abs() < 0.01);
        assert!((score.total - 0.796).abs() < 0.02);
    }

    #[test]
    fn test_relevance_score_zero() {
        let score = RelevanceScore::zero();

        assert_eq!(score.semantic, 0.0);
        assert_eq!(score.cwd_match, 0.0);
        assert_eq!(score.files_overlap, 0.0);
        assert_eq!(score.recency, 0.0);
        assert_eq!(score.total, 0.0);
    }
}
