use crate::{count_by, RoutingPolicyRecord};

pub(crate) fn routing_review_modes(policies: &[RoutingPolicyRecord]) -> String {
    count_by(policies.iter().map(|policy| policy.review_mode.as_str()))
}
