use super::{
    clustering_data::{ClaimEntityFeatureRow, ProviderPeerFeatureRow},
    clustering_types::{
        ProviderGraphAnomalyCandidate, UnsupervisedFactorRank, UnsupervisedFactorRanking,
    },
    round4,
};

pub(super) fn normalize_provider_rows(rows: &[ProviderPeerFeatureRow]) -> Vec<[f64; 5]> {
    let raw = rows
        .iter()
        .map(|row| {
            [
                row.claim_count,
                row.avg_claim_amount,
                row.high_cost_rate,
                row.peer_z_score,
                row.graph_degree,
            ]
        })
        .collect::<Vec<_>>();
    let mut means = [0.0; 5];
    for values in &raw {
        for index in 0..5 {
            means[index] += values[index];
        }
    }
    for mean in &mut means {
        *mean /= raw.len() as f64;
    }
    let mut stddevs = [0.0; 5];
    for values in &raw {
        for index in 0..5 {
            stddevs[index] += (values[index] - means[index]).powi(2);
        }
    }
    for stddev in &mut stddevs {
        *stddev = (*stddev / raw.len() as f64).sqrt();
        if *stddev == 0.0 {
            *stddev = 1.0;
        }
    }
    raw.iter()
        .map(|values| {
            let mut normalized = [0.0; 5];
            for index in 0..5 {
                normalized[index] = (values[index] - means[index]) / stddevs[index];
            }
            normalized
        })
        .collect()
}

pub(super) fn normalize_claim_entity_rows(rows: &[ClaimEntityFeatureRow]) -> Vec<[f64; 9]> {
    let raw = rows
        .iter()
        .map(|row| {
            [
                row.claim_amount,
                row.amount_to_limit_ratio,
                row.peer_percentile,
                row.item_count,
                row.high_cost_item_ratio,
                row.provider_risk_tier,
                row.diagnosis_procedure_mismatch,
                row.member_degree,
                row.provider_degree,
            ]
        })
        .collect::<Vec<_>>();
    let mut means = [0.0; 9];
    for values in &raw {
        for index in 0..9 {
            means[index] += values[index];
        }
    }
    for mean in &mut means {
        *mean /= raw.len() as f64;
    }
    let mut stddevs = [0.0; 9];
    for values in &raw {
        for index in 0..9 {
            stddevs[index] += (values[index] - means[index]).powi(2);
        }
    }
    for stddev in &mut stddevs {
        *stddev = (*stddev / raw.len() as f64).sqrt();
        if *stddev == 0.0 {
            *stddev = 1.0;
        }
    }
    raw.iter()
        .map(|values| {
            let mut normalized = [0.0; 9];
            for index in 0..9 {
                normalized[index] = (values[index] - means[index]) / stddevs[index];
            }
            normalized
        })
        .collect()
}

pub(super) fn assign_provider_clusters(rows: &[[f64; 5]], cluster_count: usize) -> Vec<usize> {
    assign_standardized_clusters(rows, 3, cluster_count)
}

pub(super) fn assign_standardized_clusters<const N: usize>(
    rows: &[[f64; N]],
    ordering_feature_index: usize,
    cluster_count: usize,
) -> Vec<usize> {
    let mut ordered = rows.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.1[ordering_feature_index]
            .partial_cmp(&right.1[ordering_feature_index])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut centroids = (0..cluster_count)
        .map(|cluster_index| {
            let source_index = cluster_index * (ordered.len() - 1) / cluster_count.max(1);
            *ordered[source_index].1
        })
        .collect::<Vec<_>>();
    let mut assignments = vec![0; rows.len()];
    for _ in 0..12 {
        for (row_index, row) in rows.iter().enumerate() {
            assignments[row_index] = nearest_centroid(row, &centroids);
        }
        let mut sums = vec![[0.0; N]; cluster_count];
        let mut counts = vec![0_usize; cluster_count];
        for (row, cluster_id) in rows.iter().zip(assignments.iter()) {
            counts[*cluster_id] += 1;
            for index in 0..N {
                sums[*cluster_id][index] += row[index];
            }
        }
        for cluster_id in 0..cluster_count {
            if counts[cluster_id] == 0 {
                continue;
            }
            for index in 0..N {
                centroids[cluster_id][index] = sums[cluster_id][index] / counts[cluster_id] as f64;
            }
        }
    }
    assignments
}

fn nearest_centroid<const N: usize>(row: &[f64; N], centroids: &[[f64; N]]) -> usize {
    centroids
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            squared_distance(row, *left)
                .partial_cmp(&squared_distance(row, *right))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

pub(super) fn cluster_distances(
    rows: &[[f64; 5]],
    assignments: &[usize],
    cluster_count: usize,
) -> Vec<f64> {
    standardized_cluster_distances(rows, assignments, cluster_count)
}

pub(super) fn standardized_cluster_distances<const N: usize>(
    rows: &[[f64; N]],
    assignments: &[usize],
    cluster_count: usize,
) -> Vec<f64> {
    let mut sums = vec![[0.0; N]; cluster_count];
    let mut counts = vec![0_usize; cluster_count];
    for (row, cluster_id) in rows.iter().zip(assignments.iter()) {
        counts[*cluster_id] += 1;
        for index in 0..N {
            sums[*cluster_id][index] += row[index];
        }
    }
    let mut centroids = vec![[0.0; N]; cluster_count];
    for cluster_id in 0..cluster_count {
        if counts[cluster_id] == 0 {
            continue;
        }
        for index in 0..N {
            centroids[cluster_id][index] = sums[cluster_id][index] / counts[cluster_id] as f64;
        }
    }
    rows.iter()
        .zip(assignments.iter())
        .map(|(row, cluster_id)| squared_distance(row, &centroids[*cluster_id]).sqrt())
        .collect()
}

pub(super) fn standardized_factor_ranking<const N: usize>(
    report_kind: &str,
    feature_columns: &[String],
    rows: &[[f64; N]],
    assignments: &[usize],
    cluster_count: usize,
    anomaly_indexes: &[usize],
) -> UnsupervisedFactorRanking {
    let mut sums = vec![[0.0; N]; cluster_count];
    let mut counts = vec![0_usize; cluster_count];
    for (row, cluster_id) in rows.iter().zip(assignments.iter()) {
        counts[*cluster_id] += 1;
        for index in 0..N {
            sums[*cluster_id][index] += row[index];
        }
    }
    let mut centroids = vec![[0.0; N]; cluster_count];
    for cluster_id in 0..cluster_count {
        if counts[cluster_id] == 0 {
            continue;
        }
        for index in 0..N {
            centroids[cluster_id][index] = sums[cluster_id][index] / counts[cluster_id] as f64;
        }
    }

    let mut contribution_totals = vec![0.0; N];
    for row_index in anomaly_indexes {
        let row = &rows[*row_index];
        let centroid = &centroids[assignments[*row_index]];
        for index in 0..N {
            contribution_totals[index] += (row[index] - centroid[index]).abs();
        }
    }
    let divisor = anomaly_indexes.len().max(1) as f64;
    let mut ranked_factors = feature_columns
        .iter()
        .enumerate()
        .map(|(index, feature)| UnsupervisedFactorRank {
            rank: 0,
            feature: feature.clone(),
            ranking_score: round4(contribution_totals[index] / divisor),
            anomaly_candidate_count: anomaly_indexes.len(),
            average_abs_centroid_deviation: round4(contribution_totals[index] / divisor),
        })
        .collect::<Vec<_>>();
    ranked_factors.sort_by(|left, right| {
        right
            .ranking_score
            .partial_cmp(&left.ranking_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.feature.cmp(&right.feature))
    });
    for (index, factor) in ranked_factors.iter_mut().enumerate() {
        factor.rank = index + 1;
    }
    UnsupervisedFactorRanking {
        report_kind: report_kind.into(),
        ranking_policy:
            "average_absolute_standardized_anomaly_deviation_from_assigned_cluster_centroid".into(),
        ranked_factor_count: ranked_factors.len(),
        ranked_factors,
    }
}

pub(super) fn provider_graph_factor_ranking(
    anomaly_candidates: &[ProviderGraphAnomalyCandidate],
) -> UnsupervisedFactorRanking {
    let count = anomaly_candidates.len();
    let graph_degree = anomaly_candidates
        .iter()
        .map(|candidate| candidate.graph_degree.abs())
        .sum::<f64>()
        / count.max(1) as f64;
    let peer_z_score = anomaly_candidates
        .iter()
        .map(|candidate| candidate.peer_z_score.abs())
        .sum::<f64>()
        / count.max(1) as f64;
    let mut ranked_factors = vec![
        UnsupervisedFactorRank {
            rank: 0,
            feature: "graph_degree".into(),
            ranking_score: round4(graph_degree),
            anomaly_candidate_count: count,
            average_abs_centroid_deviation: round4(graph_degree),
        },
        UnsupervisedFactorRank {
            rank: 0,
            feature: "peer_z_score".into(),
            ranking_score: round4(peer_z_score),
            anomaly_candidate_count: count,
            average_abs_centroid_deviation: round4(peer_z_score),
        },
    ];
    ranked_factors.sort_by(|left, right| {
        right
            .ranking_score
            .partial_cmp(&left.ranking_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.feature.cmp(&right.feature))
    });
    for (index, factor) in ranked_factors.iter_mut().enumerate() {
        factor.rank = index + 1;
    }
    UnsupervisedFactorRanking {
        report_kind: "provider_graph_unsupervised_factor_ranking".into(),
        ranking_policy: "average_absolute_graph_anomaly_signal_for_review_candidates".into(),
        ranked_factor_count: ranked_factors.len(),
        ranked_factors,
    }
}

fn squared_distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| (*left - *right).powi(2))
        .sum()
}

pub(super) fn anomaly_threshold(distances: &[f64]) -> f64 {
    let mean = distances.iter().sum::<f64>() / distances.len() as f64;
    let variance = distances
        .iter()
        .map(|distance| (distance - mean).powi(2))
        .sum::<f64>()
        / distances.len() as f64;
    let threshold = mean + variance.sqrt();
    if distances.iter().any(|distance| *distance >= threshold) {
        threshold
    } else {
        distances
            .iter()
            .copied()
            .fold(0.0, |current, distance| current.max(distance))
    }
}
