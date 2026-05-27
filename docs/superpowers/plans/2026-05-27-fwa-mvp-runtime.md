# FWA MVP Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the production-style Rust modular monolith skeleton and the first executable FWA scoring path from TPA request to audit-backed response and Runtime Scoring UI.

**Architecture:** The implementation creates the full workspace boundary from the approved design, but fills only the MVP scoring path with real behavior. Core Rust crates remain pure domain logic; HTTP, SQLx, Postgres, Python service calls, and frontend API calls stay in app/adapter layers.

**Tech Stack:** Rust workspace with Axum, Tokio, SQLx, Serde, thiserror, uuid/ulid; Python FastAPI ML service; PostgreSQL migrations; React + TypeScript + Vite + TanStack Query; GitHub Actions CI.

---

## Scope

This plan implements:

- Rust workspace and crate boundaries.
- Core domain types.
- Feature calculation.
- Rule DSL evaluation.
- Model runtime contract with heuristic scorer and Python HTTP scorer boundary.
- Score aggregation.
- Audit event creation.
- PostgreSQL schema for core business, rules, model versions, scoring runs, feature values, rule runs, model scores, and audit events.
- `POST /api/v1/claims/score`.
- Python FastAPI `/score` baseline service.
- Operations Studio skeleton with real Runtime Scoring page.
- CI upgrade for Rust, Python, frontend, and repository health.

This plan does not implement:

- Agent summaries.
- QA workflow.
- Knowledge vector search.
- Rule Sandbox UI.
- Model drift monitoring.
- ROI attribution.
- Production ONNX/CUDA runtime.
- Full RBAC.

## File Structure

Create:

- `Cargo.toml`: Rust workspace root.
- `rust-toolchain.toml`: pinned stable toolchain.
- `crates/fwa-core/Cargo.toml`
- `crates/fwa-core/src/lib.rs`
- `crates/fwa-core/src/ids.rs`
- `crates/fwa-core/src/money.rs`
- `crates/fwa-core/src/risk.rs`
- `crates/fwa-core/src/domain.rs`
- `crates/fwa-features/Cargo.toml`
- `crates/fwa-features/src/lib.rs`
- `crates/fwa-rules/Cargo.toml`
- `crates/fwa-rules/src/lib.rs`
- `crates/fwa-ml-runtime/Cargo.toml`
- `crates/fwa-ml-runtime/src/lib.rs`
- `crates/fwa-scoring/Cargo.toml`
- `crates/fwa-scoring/src/lib.rs`
- `crates/fwa-audit/Cargo.toml`
- `crates/fwa-audit/src/lib.rs`
- `crates/fwa-auth/Cargo.toml`
- `crates/fwa-auth/src/lib.rs`
- `crates/fwa-connectors/Cargo.toml`
- `crates/fwa-connectors/src/lib.rs`
- `crates/fwa-agent/Cargo.toml`
- `crates/fwa-agent/src/lib.rs`
- `apps/api-server/Cargo.toml`
- `apps/api-server/src/lib.rs`
- `apps/api-server/src/main.rs`
- `apps/api-server/src/app.rs`
- `apps/api-server/src/config.rs`
- `apps/api-server/src/error.rs`
- `apps/api-server/src/routes/mod.rs`
- `apps/api-server/src/routes/health.rs`
- `apps/api-server/src/routes/claims.rs`
- `apps/api-server/src/repository.rs`
- `apps/api-server/tests/claims_score.rs`
- `apps/worker/Cargo.toml`
- `apps/worker/src/main.rs`
- `apps/ml-service/pyproject.toml`
- `apps/ml-service/app/main.py`
- `apps/ml-service/app/schemas.py`
- `apps/ml-service/app/scorer.py`
- `apps/ml-service/tests/test_score.py`
- `apps/web-console/package.json`
- `apps/web-console/index.html`
- `apps/web-console/src/main.tsx`
- `apps/web-console/src/App.tsx`
- `apps/web-console/src/api.ts`
- `apps/web-console/src/pages/RuntimeScoring.tsx`
- `apps/web-console/src/pages/PlannedModulePage.tsx`
- `apps/web-console/src/styles.css`
- `migrations/0001_initial.sql`
- `infra/docker-compose.yml`

Modify:

- `.github/workflows/ci.yml`: add Rust, Python, and frontend checks.
- `scripts/ci/check_repo.sh`: include workspace file checks.
- `README.md`: add local development commands.

---

### Task 1: Create Rust Workspace Skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: each crate/app `Cargo.toml` and `src/lib.rs` or `src/main.rs` listed in File Structure
- Modify: `scripts/ci/check_repo.sh`

- [ ] **Step 1: Write the failing workspace health expectation**

Add these lines to `scripts/ci/check_repo.sh` after the current `required_files` list:

```bash
workspace_files=(
  "Cargo.toml"
  "rust-toolchain.toml"
  "crates/fwa-core/Cargo.toml"
  "crates/fwa-features/Cargo.toml"
  "crates/fwa-rules/Cargo.toml"
  "crates/fwa-ml-runtime/Cargo.toml"
  "crates/fwa-scoring/Cargo.toml"
  "crates/fwa-audit/Cargo.toml"
  "crates/fwa-auth/Cargo.toml"
  "crates/fwa-connectors/Cargo.toml"
  "crates/fwa-agent/Cargo.toml"
  "apps/api-server/Cargo.toml"
  "apps/worker/Cargo.toml"
)

for path in "${workspace_files[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing workspace file: $path" >&2
    exit 1
  fi
done
```

- [ ] **Step 2: Run health check to verify it fails**

Run:

```bash
bash scripts/ci/check_repo.sh
```

Expected: FAIL with `missing workspace file: Cargo.toml`.

- [ ] **Step 3: Create root workspace files**

Create `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "apps/api-server",
  "apps/worker",
  "crates/fwa-core",
  "crates/fwa-features",
  "crates/fwa-rules",
  "crates/fwa-ml-runtime",
  "crates/fwa-scoring",
  "crates/fwa-audit",
  "crates/fwa-auth",
  "crates/fwa-connectors",
  "crates/fwa-agent",
]

[workspace.package]
edition = "2021"
license = "UNLICENSED"
publish = false
rust-version = "1.82"

[workspace.dependencies]
anyhow = "1"
async-trait = "0.1"
axum = "0.7"
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", features = ["json"] }
rust_decimal = { version = "1", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "json", "rust_decimal"] }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
ulid = { version = "1", features = ["serde"] }
uuid = { version = "1", features = ["serde", "v7"] }
```

Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 4: Create minimal crate manifests and entrypoints**

For each library crate, create `Cargo.toml` with its package name and dependencies.

Example for `crates/fwa-core/Cargo.toml`:

```toml
[package]
name = "fwa-core"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
chrono.workspace = true
rust_decimal.workspace = true
serde.workspace = true
thiserror.workspace = true
ulid.workspace = true
uuid.workspace = true
```

Example empty library entrypoint for each library crate:

```rust
pub fn crate_ready() -> bool {
    true
}
```

Create `apps/api-server/Cargo.toml`:

```toml
[package]
name = "api-server"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
anyhow.workspace = true
axum.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

Create `apps/api-server/src/main.rs`:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    tracing::info!("api-server skeleton ready");
    Ok(())
}
```

Create `apps/worker/Cargo.toml`:

```toml
[package]
name = "worker"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
anyhow.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

Create `apps/worker/src/main.rs`:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    tracing::info!("worker skeleton ready");
    Ok(())
}
```

- [ ] **Step 5: Run workspace checks**

Run:

```bash
bash scripts/ci/check_repo.sh
cargo fmt --all -- --check
cargo test --workspace
```

Expected: all commands PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml rust-toolchain.toml crates apps scripts/ci/check_repo.sh
git commit -m "chore: scaffold Rust workspace"
```

---

### Task 2: Add Core Domain Types

**Files:**
- Modify: `crates/fwa-core/src/lib.rs`
- Create: `crates/fwa-core/src/ids.rs`
- Create: `crates/fwa-core/src/money.rs`
- Create: `crates/fwa-core/src/risk.rs`
- Create: `crates/fwa-core/src/domain.rs`

- [ ] **Step 1: Replace `fwa-core` skeleton with module exports**

Edit `crates/fwa-core/src/lib.rs`:

```rust
pub mod domain;
pub mod ids;
pub mod money;
pub mod risk;

pub use domain::*;
pub use ids::*;
pub use money::*;
pub use risk::*;
```

- [ ] **Step 2: Add ID types with tests**

Create `crates/fwa-core/src/ids.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use ulid::Ulid;

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, Ulid::new()))
            }

            pub fn from_external(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(ClaimId, "claim");
id_type!(MemberId, "member");
id_type!(PolicyId, "policy");
id_type!(ProviderId, "provider");
id_type!(ScoringRunId, "run");
id_type!(AuditEventId, "aud");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_run_id_has_expected_prefix() {
        let id = ScoringRunId::new();
        assert!(id.as_str().starts_with("run_"));
    }

    #[test]
    fn external_claim_id_is_preserved() {
        let id = ClaimId::from_external("CLM-0287");
        assert_eq!(id.as_str(), "CLM-0287");
    }
}
```

- [ ] **Step 3: Add Money and Risk types**

Create `crates/fwa-core/src/money.rs`:

```rust
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Money {
    pub amount: Decimal,
    pub currency: String,
}

impl Money {
    pub fn new(amount: Decimal, currency: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into(),
        }
    }
}
```

Create `crates/fwa-core/src/risk.rs`:

```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RiskScoreError {
    #[error("risk score must be between 0 and 100")]
    OutOfRange,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskScore(u8);

impl RiskScore {
    pub fn new(value: u8) -> Result<Self, RiskScoreError> {
        if value <= 100 {
            Ok(Self(value))
        } else {
            Err(RiskScoreError::OutOfRange)
        }
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Green,
    Amber,
    Red,
}

impl RiskLevel {
    pub fn from_score(score: RiskScore) -> Self {
        match score.value() {
            0..=39 => Self::Green,
            40..=69 => Self::Amber,
            _ => Self::Red,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendedAction {
    AutoApprove,
    ManualReview,
    EscalateInvestigation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_scores_to_rag_levels() {
        assert_eq!(RiskLevel::from_score(RiskScore::new(20).unwrap()), RiskLevel::Green);
        assert_eq!(RiskLevel::from_score(RiskScore::new(55).unwrap()), RiskLevel::Amber);
        assert_eq!(RiskLevel::from_score(RiskScore::new(87).unwrap()), RiskLevel::Red);
    }
}
```

- [ ] **Step 4: Add domain structs**

Create `crates/fwa-core/src/domain.rs`:

```rust
use crate::{ClaimId, MemberId, Money, PolicyId, ProviderId};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: MemberId,
    pub external_member_id: String,
    pub dob: Option<NaiveDate>,
    pub gender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub external_policy_id: String,
    pub member_id: MemberId,
    pub product_code: String,
    pub coverage_start_date: NaiveDate,
    pub coverage_end_date: NaiveDate,
    pub coverage_limit: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub external_provider_id: String,
    pub name: String,
    pub provider_type: String,
    pub region: String,
    pub risk_tier: ProviderRiskTier,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderRiskTier {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: ClaimId,
    pub external_claim_id: String,
    pub member_id: MemberId,
    pub policy_id: PolicyId,
    pub provider_id: ProviderId,
    pub diagnosis_code: String,
    pub service_date: NaiveDate,
    pub amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimItem {
    pub item_code: String,
    pub item_type: String,
    pub description: String,
    pub quantity: u32,
    pub total_amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimContext {
    pub claim: Claim,
    pub items: Vec<ClaimItem>,
    pub member: Member,
    pub policy: Policy,
    pub provider: Provider,
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p fwa-core
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/fwa-core
git commit -m "feat: add FWA core domain types"
```

---

### Task 3: Implement Feature Calculation

**Files:**
- Modify: `crates/fwa-features/Cargo.toml`
- Modify: `crates/fwa-features/src/lib.rs`

- [ ] **Step 1: Add dependencies**

Edit `crates/fwa-features/Cargo.toml`:

```toml
[package]
name = "fwa-features"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
chrono.workspace = true
fwa-core = { path = "../fwa-core" }
rust_decimal.workspace = true
serde.workspace = true
serde_json.workspace = true
```

- [ ] **Step 2: Add feature calculator with tests**

Replace `crates/fwa-features/src/lib.rs`:

```rust
use fwa_core::{ClaimContext, ProviderRiskTier};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceRef {
    pub entity_type: String,
    pub entity_id: String,
    pub field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureValue {
    pub name: String,
    pub version: u16,
    pub value: Value,
    pub evidence_refs: Vec<EvidenceRef>,
}

pub type FeatureMap = BTreeMap<String, FeatureValue>;

pub fn calculate_features(context: &ClaimContext) -> FeatureMap {
    let mut features = FeatureMap::new();
    let claim_id = context.claim.external_claim_id.clone();

    let days_since_policy_start =
        (context.claim.service_date - context.policy.coverage_start_date).num_days();
    insert_number(
        &mut features,
        "days_since_policy_start",
        days_since_policy_start,
        &claim_id,
        "service_date",
    );

    let claim_amount = context.claim.amount.amount;
    let limit = context.policy.coverage_limit.amount;
    let ratio = if limit.is_zero() {
        0.0
    } else {
        (claim_amount / limit).to_f64().unwrap_or(0.0)
    };
    insert_number(
        &mut features,
        "claim_amount_to_limit_ratio",
        ratio,
        &claim_id,
        "claim_amount",
    );

    insert_number(
        &mut features,
        "claim_item_count",
        context.items.len() as i64,
        &claim_id,
        "claim_items",
    );

    let provider_risk = match context.provider.risk_tier {
        ProviderRiskTier::Low => "LOW",
        ProviderRiskTier::Medium => "MEDIUM",
        ProviderRiskTier::High => "HIGH",
    };
    insert_string(
        &mut features,
        "provider_risk_tier",
        provider_risk,
        &context.provider.external_provider_id,
        "risk_tier",
    );

    features
}

fn insert_number(
    features: &mut FeatureMap,
    name: &str,
    value: impl serde::Serialize,
    entity_id: &str,
    field: &str,
) {
    features.insert(
        name.to_string(),
        FeatureValue {
            name: name.to_string(),
            version: 1,
            value: serde_json::to_value(value).expect("feature value serializes"),
            evidence_refs: vec![EvidenceRef {
                entity_type: "claim".to_string(),
                entity_id: entity_id.to_string(),
                field: field.to_string(),
            }],
        },
    );
}

fn insert_string(
    features: &mut FeatureMap,
    name: &str,
    value: &str,
    entity_id: &str,
    field: &str,
) {
    features.insert(
        name.to_string(),
        FeatureValue {
            name: name.to_string(),
            version: 1,
            value: Value::String(value.to_string()),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".to_string(),
                entity_id: entity_id.to_string(),
                field: field.to_string(),
            }],
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_core::*;
    use rust_decimal::Decimal;

    fn context() -> ClaimContext {
        let member_id = MemberId::from_external("MBR-1");
        let policy_id = PolicyId::from_external("POL-1");
        let provider_id = ProviderId::from_external("PRV-1");
        ClaimContext {
            claim: Claim {
                id: ClaimId::from_external("CLM-1"),
                external_claim_id: "CLM-1".into(),
                member_id: member_id.clone(),
                policy_id: policy_id.clone(),
                provider_id: provider_id.clone(),
                diagnosis_code: "J10".into(),
                service_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
                amount: Money::new(Decimal::new(8000, 0), "CNY"),
            },
            items: vec![],
            member: Member {
                id: member_id.clone(),
                external_member_id: "MBR-1".into(),
                dob: None,
                gender: None,
            },
            policy: Policy {
                id: policy_id,
                external_policy_id: "POL-1".into(),
                member_id,
                product_code: "MED".into(),
                coverage_start_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                coverage_end_date: chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
                coverage_limit: Money::new(Decimal::new(10000, 0), "CNY"),
            },
            provider: Provider {
                id: provider_id,
                external_provider_id: "PRV-1".into(),
                name: "Demo Hospital".into(),
                provider_type: "hospital".into(),
                region: "SH".into(),
                risk_tier: ProviderRiskTier::Medium,
            },
        }
    }

    #[test]
    fn calculates_policy_age_and_amount_ratio() {
        let features = calculate_features(&context());
        assert_eq!(features["days_since_policy_start"].value, serde_json::json!(5));
        assert_eq!(features["claim_amount_to_limit_ratio"].value, serde_json::json!(0.8));
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p fwa-features
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/fwa-features
git commit -m "feat: calculate MVP FWA features"
```

---

### Task 4: Implement Rule DSL Evaluation

**Files:**
- Modify: `crates/fwa-rules/Cargo.toml`
- Modify: `crates/fwa-rules/src/lib.rs`

- [ ] **Step 1: Add dependencies**

Edit `crates/fwa-rules/Cargo.toml`:

```toml
[package]
name = "fwa-rules"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
fwa-core = { path = "../fwa-core" }
fwa-features = { path = "../fwa-features" }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
```

- [ ] **Step 2: Add rule engine**

Replace `crates/fwa-rules/src/lib.rs`:

```rust
use fwa_core::RecommendedAction;
use fwa_features::FeatureMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("unsupported operator: {0}")]
    UnsupportedOperator(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub rule_id: String,
    pub version: u32,
    pub name: String,
    pub conditions: Vec<Condition>,
    pub action: RuleAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    pub score: u8,
    pub alert_code: String,
    pub recommended_action: RecommendedAction,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleMatch {
    pub rule_id: String,
    pub rule_version: u32,
    pub score_contribution: u8,
    pub alert_code: String,
    pub reason: String,
    pub recommended_action: RecommendedAction,
}

pub fn evaluate_rules(rules: &[Rule], features: &FeatureMap) -> Result<Vec<RuleMatch>, RuleError> {
    let mut matches = Vec::new();
    for rule in rules {
        let matched = rule
            .conditions
            .iter()
            .map(|condition| evaluate_condition(condition, features))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .all(|value| value);

        if matched {
            matches.push(RuleMatch {
                rule_id: rule.rule_id.clone(),
                rule_version: rule.version,
                score_contribution: rule.action.score,
                alert_code: rule.action.alert_code.clone(),
                reason: rule.action.reason.clone(),
                recommended_action: rule.action.recommended_action,
            });
        }
    }
    Ok(matches)
}

fn evaluate_condition(condition: &Condition, features: &FeatureMap) -> Result<bool, RuleError> {
    let Some(feature) = features.get(&condition.field) else {
        return Ok(false);
    };

    match condition.operator.as_str() {
        "<=" => Ok(as_f64(&feature.value) <= as_f64(&condition.value)),
        ">=" => Ok(as_f64(&feature.value) >= as_f64(&condition.value)),
        "==" => Ok(feature.value == condition.value),
        other => Err(RuleError::UnsupportedOperator(other.to_string())),
    }
}

fn as_f64(value: &Value) -> f64 {
    value.as_f64().or_else(|| value.as_i64().map(|v| v as f64)).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_features::FeatureValue;
    use std::collections::BTreeMap;

    #[test]
    fn matches_rule_when_all_conditions_match() {
        let mut features = BTreeMap::new();
        features.insert(
            "days_since_policy_start".into(),
            FeatureValue {
                name: "days_since_policy_start".into(),
                version: 1,
                value: serde_json::json!(5),
                evidence_refs: vec![],
            },
        );

        let rules = vec![Rule {
            rule_id: "rule_early_claim".into(),
            version: 1,
            name: "Early claim".into(),
            conditions: vec![Condition {
                field: "days_since_policy_start".into(),
                operator: "<=".into(),
                value: serde_json::json!(7),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "EARLY_CLAIM".into(),
                recommended_action: RecommendedAction::ManualReview,
                reason: "保单生效后 7 天内发生理赔".into(),
            },
        }];

        let matches = evaluate_rules(&rules, &features).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].alert_code, "EARLY_CLAIM");
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p fwa-rules
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/fwa-rules
git commit -m "feat: add rule DSL evaluation"
```

---

### Task 5: Implement Model Runtime Contract

**Files:**
- Modify: `crates/fwa-ml-runtime/Cargo.toml`
- Modify: `crates/fwa-ml-runtime/src/lib.rs`

- [ ] **Step 1: Add dependencies**

Edit `crates/fwa-ml-runtime/Cargo.toml`:

```toml
[package]
name = "fwa-ml-runtime"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
async-trait.workspace = true
fwa-core = { path = "../fwa-core" }
fwa-features = { path = "../fwa-features" }
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
```

- [ ] **Step 2: Add scorer trait and heuristic implementation**

Replace `crates/fwa-ml-runtime/src/lib.rs`:

```rust
use async_trait::async_trait;
use fwa_core::{ClaimId, ScoringRunId};
use fwa_features::FeatureMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelRuntimeError {
    #[error("model service unavailable")]
    ServiceUnavailable,
    #[error("model response invalid: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelScoreRequest {
    pub run_id: ScoringRunId,
    pub claim_id: ClaimId,
    pub model_key: String,
    pub features: FeatureMap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelExplanation {
    pub feature: String,
    pub direction: String,
    pub contribution: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelScore {
    pub model_key: String,
    pub model_version: String,
    pub runtime_kind: String,
    pub execution_provider: String,
    pub score: u8,
    pub label: String,
    pub explanations: Vec<ModelExplanation>,
    pub latency_ms: u64,
}

#[async_trait]
pub trait ModelScorer: Send + Sync {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError>;
}

#[derive(Debug, Default)]
pub struct HeuristicModelScorer;

#[async_trait]
impl ModelScorer for HeuristicModelScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        let ratio = request
            .features
            .get("claim_amount_to_limit_ratio")
            .and_then(|feature| feature.value.as_f64())
            .unwrap_or(0.0);
        let score = (ratio * 100.0).round().clamp(0.0, 100.0) as u8;
        Ok(ModelScore {
            model_key: request.model_key,
            model_version: "heuristic-0.1.0".into(),
            runtime_kind: "heuristic".into(),
            execution_provider: "cpu".into(),
            score,
            label: if score >= 70 { "HIGH_RISK" } else { "LOW_RISK" }.into(),
            explanations: vec![ModelExplanation {
                feature: "claim_amount_to_limit_ratio".into(),
                direction: "increases_risk".into(),
                contribution: ratio,
                reason: "理赔金额占保障额度比例影响模型分".into(),
            }],
            latency_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_features::FeatureValue;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn heuristic_scorer_maps_amount_ratio_to_score() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_to_limit_ratio".into(),
            FeatureValue {
                name: "claim_amount_to_limit_ratio".into(),
                version: 1,
                value: serde_json::json!(0.82),
                evidence_refs: vec![],
            },
        );

        let scorer = HeuristicModelScorer;
        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_test"),
                claim_id: ClaimId::from_external("CLM-1"),
                model_key: "baseline_fwa".into(),
                features,
            })
            .await
            .unwrap();

        assert_eq!(result.score, 82);
        assert_eq!(result.runtime_kind, "heuristic");
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p fwa-ml-runtime
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/fwa-ml-runtime
git commit -m "feat: define model runtime contract"
```

---

### Task 6: Implement Score Aggregation

**Files:**
- Modify: `crates/fwa-scoring/Cargo.toml`
- Modify: `crates/fwa-scoring/src/lib.rs`

- [ ] **Step 1: Add dependencies**

Edit `crates/fwa-scoring/Cargo.toml`:

```toml
[package]
name = "fwa-scoring"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
fwa-core = { path = "../fwa-core" }
fwa-ml-runtime = { path = "../fwa-ml-runtime" }
fwa-rules = { path = "../fwa-rules" }
serde.workspace = true
```

- [ ] **Step 2: Add aggregation logic**

Replace `crates/fwa-scoring/src/lib.rs`:

```rust
use fwa_core::{RecommendedAction, RiskLevel, RiskScore};
use fwa_ml_runtime::ModelScore;
use fwa_rules::RuleMatch;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoringDecision {
    pub risk_score: RiskScore,
    pub rag: RiskLevel,
    pub recommended_action: RecommendedAction,
    pub rule_score: u8,
    pub ml_score: u8,
    pub top_reasons: Vec<String>,
}

pub fn aggregate(rule_matches: &[RuleMatch], model_score: &ModelScore) -> ScoringDecision {
    let rule_score = rule_matches
        .iter()
        .map(|rule_match| rule_match.score_contribution)
        .sum::<u8>()
        .min(100);
    let final_score_value = ((rule_score as u16 + model_score.score as u16) / 2) as u8;
    let risk_score = RiskScore::new(final_score_value).expect("clamped score is valid");
    let rag = RiskLevel::from_score(risk_score);
    let recommended_action = match rag {
        RiskLevel::Green => RecommendedAction::AutoApprove,
        RiskLevel::Amber => RecommendedAction::ManualReview,
        RiskLevel::Red => RecommendedAction::EscalateInvestigation,
    };
    let mut top_reasons: Vec<String> = rule_matches
        .iter()
        .map(|rule_match| rule_match.reason.clone())
        .collect();
    top_reasons.extend(model_score.explanations.iter().map(|explanation| explanation.reason.clone()));
    top_reasons.truncate(5);

    ScoringDecision {
        risk_score,
        rag,
        recommended_action,
        rule_score,
        ml_score: model_score.score,
        top_reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_ml_runtime::ModelExplanation;

    #[test]
    fn aggregates_rule_and_model_scores() {
        let rules = vec![RuleMatch {
            rule_id: "rule_1".into(),
            rule_version: 1,
            score_contribution: 80,
            alert_code: "EARLY_HIGH_AMOUNT".into(),
            reason: "早期高额理赔".into(),
            recommended_action: RecommendedAction::ManualReview,
        }];
        let model = ModelScore {
            model_key: "baseline".into(),
            model_version: "0.1.0".into(),
            runtime_kind: "heuristic".into(),
            execution_provider: "cpu".into(),
            score: 90,
            label: "HIGH_RISK".into(),
            explanations: vec![ModelExplanation {
                feature: "claim_amount_to_limit_ratio".into(),
                direction: "increases_risk".into(),
                contribution: 0.8,
                reason: "金额比例高".into(),
            }],
            latency_ms: 0,
        };

        let decision = aggregate(&rules, &model);
        assert_eq!(decision.risk_score.value(), 85);
        assert_eq!(decision.rag, RiskLevel::Red);
        assert_eq!(decision.recommended_action, RecommendedAction::EscalateInvestigation);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p fwa-scoring
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/fwa-scoring
git commit -m "feat: aggregate FWA risk scores"
```

---

### Task 7: Implement Audit Event Creation

**Files:**
- Modify: `crates/fwa-audit/Cargo.toml`
- Modify: `crates/fwa-audit/src/lib.rs`

- [ ] **Step 1: Add dependencies**

Edit `crates/fwa-audit/Cargo.toml`:

```toml
[package]
name = "fwa-audit"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
chrono.workspace = true
fwa-core = { path = "../fwa-core" }
serde.workspace = true
serde_json.workspace = true
```

- [ ] **Step 2: Add audit event type**

Replace `crates/fwa-audit/src/lib.rs`:

```rust
use chrono::{DateTime, Utc};
use fwa_core::{AuditEventId, ClaimId, ScoringRunId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActorContext {
    pub actor_id: String,
    pub actor_role: String,
    pub source_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditEventStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEvent {
    pub audit_id: AuditEventId,
    pub run_id: ScoringRunId,
    pub claim_id: ClaimId,
    pub actor: ActorContext,
    pub event_type: String,
    pub event_status: AuditEventStatus,
    pub summary: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

pub fn scoring_completed(
    run_id: ScoringRunId,
    claim_id: ClaimId,
    actor: ActorContext,
    payload: Value,
) -> AuditEvent {
    AuditEvent {
        audit_id: AuditEventId::new(),
        run_id,
        claim_id,
        actor,
        event_type: "scoring.completed".into(),
        event_status: AuditEventStatus::Succeeded,
        summary: "FWA scoring completed".into(),
        payload,
        created_at: Utc::now(),
    }
}

pub fn scoring_failed(
    run_id: ScoringRunId,
    claim_id: ClaimId,
    actor: ActorContext,
    payload: Value,
) -> AuditEvent {
    AuditEvent {
        audit_id: AuditEventId::new(),
        run_id,
        claim_id,
        actor,
        event_type: "scoring.failed".into(),
        event_status: AuditEventStatus::Failed,
        summary: "FWA scoring failed".into(),
        payload,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_event_contains_run_and_claim_ids() {
        let event = scoring_completed(
            ScoringRunId::from_external("run_1"),
            ClaimId::from_external("CLM-1"),
            ActorContext {
                actor_id: "tpa-demo".into(),
                actor_role: "system".into(),
                source_system: "tpa-demo".into(),
            },
            serde_json::json!({"risk_score": 80}),
        );

        assert_eq!(event.run_id.as_str(), "run_1");
        assert_eq!(event.claim_id.as_str(), "CLM-1");
        assert_eq!(event.event_status, AuditEventStatus::Succeeded);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p fwa-audit
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/fwa-audit
git commit -m "feat: add audit event model"
```

---

### Task 8: Add API Key Auth

**Files:**
- Modify: `crates/fwa-auth/Cargo.toml`
- Modify: `crates/fwa-auth/src/lib.rs`

- [ ] **Step 1: Add dependencies**

Edit `crates/fwa-auth/Cargo.toml`:

```toml
[package]
name = "fwa-auth"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
fwa-audit = { path = "../fwa-audit" }
thiserror.workspace = true
```

- [ ] **Step 2: Add validator**

Replace `crates/fwa-auth/src/lib.rs`:

```rust
use fwa_audit::ActorContext;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("invalid api key")]
    InvalidApiKey,
}

#[derive(Debug, Clone)]
pub struct ApiKeyConfig {
    pub key: String,
    pub source_system: String,
}

pub fn validate_api_key(
    provided_key: Option<&str>,
    config: &ApiKeyConfig,
) -> Result<ActorContext, AuthError> {
    match provided_key {
        Some(value) if value == config.key => Ok(ActorContext {
            actor_id: config.source_system.clone(),
            actor_role: "tpa_system".into(),
            source_system: config.source_system.clone(),
        }),
        _ => Err(AuthError::InvalidApiKey),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_key_returns_actor_context() {
        let config = ApiKeyConfig {
            key: "secret".into(),
            source_system: "tpa-demo".into(),
        };
        let actor = validate_api_key(Some("secret"), &config).unwrap();
        assert_eq!(actor.source_system, "tpa-demo");
    }

    #[test]
    fn invalid_key_is_rejected() {
        let config = ApiKeyConfig {
            key: "secret".into(),
            source_system: "tpa-demo".into(),
        };
        assert_eq!(
            validate_api_key(Some("wrong"), &config).unwrap_err(),
            AuthError::InvalidApiKey
        );
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p fwa-auth
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/fwa-auth
git commit -m "feat: add API key auth"
```

---

### Task 9: Add PostgreSQL Migration

**Files:**
- Create: `migrations/0001_initial.sql`

- [ ] **Step 1: Create migration**

Create `migrations/0001_initial.sql`:

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE members (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  external_member_id TEXT NOT NULL UNIQUE,
  name_hash TEXT,
  dob DATE,
  gender TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE policies (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  external_policy_id TEXT NOT NULL UNIQUE,
  member_id UUID NOT NULL REFERENCES members(id),
  product_code TEXT NOT NULL,
  coverage_start_date DATE NOT NULL,
  coverage_end_date DATE NOT NULL,
  coverage_limit_amount NUMERIC NOT NULL,
  currency TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE providers (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  external_provider_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  provider_type TEXT NOT NULL,
  region TEXT NOT NULL,
  risk_tier TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE claims (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  external_claim_id TEXT NOT NULL UNIQUE,
  member_id UUID NOT NULL REFERENCES members(id),
  policy_id UUID NOT NULL REFERENCES policies(id),
  provider_id UUID NOT NULL REFERENCES providers(id),
  claim_type TEXT NOT NULL,
  diagnosis_code TEXT NOT NULL,
  service_date DATE NOT NULL,
  claim_amount NUMERIC NOT NULL,
  currency TEXT NOT NULL,
  status TEXT NOT NULL,
  raw_payload JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE claim_items (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  claim_id UUID NOT NULL REFERENCES claims(id) ON DELETE CASCADE,
  item_code TEXT NOT NULL,
  item_type TEXT NOT NULL,
  description TEXT NOT NULL,
  quantity INTEGER NOT NULL,
  unit_amount NUMERIC NOT NULL,
  total_amount NUMERIC NOT NULL,
  currency TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE rules (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  rule_key TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  owner TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE rule_versions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  rule_id UUID NOT NULL REFERENCES rules(id),
  version INTEGER NOT NULL,
  dsl JSONB NOT NULL,
  score INTEGER NOT NULL,
  recommended_action TEXT NOT NULL,
  created_by TEXT NOT NULL,
  approved_by TEXT,
  published_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE(rule_id, version)
);

CREATE TABLE model_versions (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  model_key TEXT NOT NULL,
  version TEXT NOT NULL,
  model_type TEXT NOT NULL,
  runtime_kind TEXT NOT NULL,
  artifact_uri TEXT,
  endpoint_url TEXT,
  execution_provider TEXT NOT NULL,
  status TEXT NOT NULL,
  metrics JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  activated_at TIMESTAMPTZ,
  UNIQUE(model_key, version)
);

CREATE TABLE scoring_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL UNIQUE,
  claim_id UUID REFERENCES claims(id),
  source_system TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  status TEXT NOT NULL,
  risk_score INTEGER,
  rag TEXT,
  recommended_action TEXT,
  started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  completed_at TIMESTAMPTZ,
  error_code TEXT,
  error_message TEXT
);

CREATE TABLE feature_values (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  claim_id UUID REFERENCES claims(id),
  feature_name TEXT NOT NULL,
  feature_version INTEGER NOT NULL,
  value_json JSONB NOT NULL,
  evidence_json JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE rule_runs (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  rule_id UUID REFERENCES rules(id),
  rule_version_id UUID REFERENCES rule_versions(id),
  matched BOOLEAN NOT NULL,
  score_contribution INTEGER NOT NULL,
  alert_code TEXT,
  reason TEXT,
  evidence_json JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE model_scores (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  model_version_id UUID REFERENCES model_versions(id),
  model_key TEXT NOT NULL,
  runtime_kind TEXT NOT NULL,
  execution_provider TEXT NOT NULL,
  score INTEGER NOT NULL,
  label TEXT NOT NULL,
  explanation_json JSONB NOT NULL,
  latency_ms INTEGER NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE audit_events (
  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  audit_id TEXT NOT NULL UNIQUE,
  run_id TEXT NOT NULL REFERENCES scoring_runs(run_id) ON DELETE CASCADE,
  claim_id UUID REFERENCES claims(id),
  actor_id TEXT NOT NULL,
  actor_role TEXT NOT NULL,
  source_system TEXT NOT NULL,
  event_type TEXT NOT NULL,
  event_status TEXT NOT NULL,
  summary TEXT NOT NULL,
  payload JSONB NOT NULL,
  evidence_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

- [ ] **Step 2: Run SQL parse check**

Run:

```bash
psql --version
```

Expected: command exists. If Postgres client is unavailable, skip parse locally and rely on migration check task after Docker Compose is added.

- [ ] **Step 3: Commit**

```bash
git add migrations/0001_initial.sql
git commit -m "feat: add initial database schema"
```

---

### Task 10: Add Python ML Service

**Files:**
- Create: `apps/ml-service/pyproject.toml`
- Create: `apps/ml-service/app/main.py`
- Create: `apps/ml-service/app/schemas.py`
- Create: `apps/ml-service/app/scorer.py`
- Create: `apps/ml-service/tests/test_score.py`

- [ ] **Step 1: Create Python package config**

Create `apps/ml-service/pyproject.toml`:

```toml
[project]
name = "fwa-ml-service"
version = "0.1.0"
requires-python = ">=3.12"
dependencies = [
  "fastapi>=0.115",
  "pydantic>=2",
  "uvicorn>=0.34",
]

[project.optional-dependencies]
dev = [
  "httpx>=0.28",
  "pytest>=8",
]
```

- [ ] **Step 2: Add schemas**

Create `apps/ml-service/app/schemas.py`:

```python
from pydantic import BaseModel, Field


class ScoreRequest(BaseModel):
    run_id: str
    claim_id: str
    model_key: str = "baseline_fwa"
    features: dict[str, object]


class ModelExplanation(BaseModel):
    feature: str
    direction: str
    contribution: float
    reason: str


class ScoreResponse(BaseModel):
    model_key: str
    model_version: str
    score: int = Field(ge=0, le=100)
    label: str
    explanations: list[ModelExplanation]
```

- [ ] **Step 3: Add deterministic scorer**

Create `apps/ml-service/app/scorer.py`:

```python
from .schemas import ModelExplanation, ScoreRequest, ScoreResponse


def score_claim(request: ScoreRequest) -> ScoreResponse:
    ratio = float(request.features.get("claim_amount_to_limit_ratio", 0.0))
    provider_tier = str(request.features.get("provider_risk_tier", "LOW"))
    tier_bonus = {"LOW": 0, "MEDIUM": 8, "HIGH": 18}.get(provider_tier, 0)
    score = max(0, min(100, round(ratio * 100 + tier_bonus)))
    label = "HIGH_RISK" if score >= 70 else "LOW_RISK"
    return ScoreResponse(
        model_key=request.model_key,
        model_version="0.1.0",
        score=score,
        label=label,
        explanations=[
            ModelExplanation(
                feature="claim_amount_to_limit_ratio",
                direction="increases_risk",
                contribution=ratio,
                reason="理赔金额占保障额度比例较高",
            )
        ],
    )
```

- [ ] **Step 4: Add FastAPI app and tests**

Create `apps/ml-service/app/main.py`:

```python
from fastapi import FastAPI

from .schemas import ScoreRequest, ScoreResponse
from .scorer import score_claim

app = FastAPI(title="FWA ML Service")


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/score", response_model=ScoreResponse)
def score(request: ScoreRequest) -> ScoreResponse:
    return score_claim(request)
```

Create `apps/ml-service/tests/test_score.py`:

```python
from fastapi.testclient import TestClient

from app.main import app


client = TestClient(app)


def test_score_returns_high_risk_for_large_amount_ratio():
    response = client.post(
        "/score",
        json={
            "run_id": "run_test",
            "claim_id": "CLM-1",
            "model_key": "baseline_fwa",
            "features": {
                "claim_amount_to_limit_ratio": 0.82,
                "provider_risk_tier": "MEDIUM",
            },
        },
    )

    assert response.status_code == 200
    payload = response.json()
    assert payload["score"] == 90
    assert payload["label"] == "HIGH_RISK"
    assert payload["model_version"] == "0.1.0"
```

- [ ] **Step 5: Run tests**

```bash
cd apps/ml-service
python -m pip install -e ".[dev]"
pytest
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/ml-service
git commit -m "feat: add baseline Python ML service"
```

---

### Task 11: Add API Server Routes and In-Memory First Integration

**Files:**
- Modify: `apps/api-server/Cargo.toml`
- Create: `apps/api-server/src/app.rs`
- Create: `apps/api-server/src/config.rs`
- Create: `apps/api-server/src/error.rs`
- Create: `apps/api-server/src/routes/health.rs`
- Create: `apps/api-server/src/routes/claims.rs`
- Create: `apps/api-server/src/routes/mod.rs`
- Create: `apps/api-server/src/lib.rs`
- Modify: `apps/api-server/src/main.rs`
- Create: `apps/api-server/tests/claims_score.rs`

- [ ] **Step 1: Add API server dependencies**

Edit `apps/api-server/Cargo.toml`:

```toml
[package]
name = "api-server"
version = "0.1.0"
edition.workspace = true
publish.workspace = true

[dependencies]
anyhow.workspace = true
axum.workspace = true
chrono.workspace = true
fwa-audit = { path = "../../crates/fwa-audit" }
fwa-auth = { path = "../../crates/fwa-auth" }
fwa-core = { path = "../../crates/fwa-core" }
fwa-features = { path = "../../crates/fwa-features" }
fwa-ml-runtime = { path = "../../crates/fwa-ml-runtime" }
fwa-rules = { path = "../../crates/fwa-rules" }
fwa-scoring = { path = "../../crates/fwa-scoring" }
rust_decimal.workspace = true
serde.workspace = true
serde_json.workspace = true
tokio.workspace = true
tower-http.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

[dev-dependencies]
tower = "0.5"
```

- [ ] **Step 2: Add app state and health route**

Create `apps/api-server/src/config.rs`:

```rust
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub source_system: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: "dev-secret".into(),
            source_system: "tpa-demo".into(),
        }
    }
}
```

Create `apps/api-server/src/routes/health.rs`:

```rust
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
```

Create `apps/api-server/src/app.rs`:

```rust
use crate::config::AppConfig;
use crate::routes::{claims, health};
use axum::{routing::{get, post}, Router};
use fwa_ml_runtime::HeuristicModelScorer;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub scorer: Arc<HeuristicModelScorer>,
}

pub fn build_app(config: AppConfig) -> Router {
    let state = AppState {
        config,
        scorer: Arc::new(HeuristicModelScorer),
    };
    Router::new()
        .route("/api/v1/health", get(health::health))
        .route("/api/v1/claims/score", post(claims::score_claim))
        .with_state(state)
}
```

Create `apps/api-server/src/lib.rs`:

```rust
pub mod app;
pub mod config;
pub mod routes;
```

- [ ] **Step 3: Add claims scoring route**

Create `apps/api-server/src/routes/claims.rs`:

```rust
use crate::app::AppState;
use axum::{extract::State, http::HeaderMap, Json};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::*;
use fwa_features::calculate_features;
use fwa_ml_runtime::{ModelScoreRequest, ModelScorer};
use fwa_rules::{evaluate_rules, Condition, Rule, RuleAction};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ScoreClaimRequest {
    pub source_system: String,
    pub claim_id: Option<String>,
    pub claim: Option<FullClaimPayload>,
}

#[derive(Debug, Deserialize)]
pub struct FullClaimPayload {
    pub external_claim_id: String,
    pub claim_amount: Decimal,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct ScoreClaimResponse {
    pub run_id: String,
    pub audit_id: String,
    pub claim_id: String,
    pub risk_score: u8,
    pub rag: RiskLevel,
    pub recommended_action: RecommendedAction,
    pub top_reasons: Vec<String>,
}

pub async fn score_claim(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ScoreClaimRequest>,
) -> Result<Json<ScoreClaimResponse>, (axum::http::StatusCode, String)> {
    let api_key = headers.get("x-api-key").and_then(|value| value.to_str().ok());
    let _actor = validate_api_key(
        api_key,
        &ApiKeyConfig {
            key: state.config.api_key.clone(),
            source_system: state.config.source_system.clone(),
        },
    )
    .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "INVALID_API_KEY".into()))?;

    if request.claim_id.is_some() == request.claim.is_some() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_SCORE_REQUEST".into(),
        ));
    }

    let context = demo_context(request.claim_id, request.claim);
    let features = calculate_features(&context);
    let rules = demo_rules();
    let rule_matches = evaluate_rules(&rules, &features)
        .map_err(|error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let run_id = ScoringRunId::new();
    let model_score = state
        .scorer
        .score(ModelScoreRequest {
            run_id: run_id.clone(),
            claim_id: context.claim.id.clone(),
            model_key: "baseline_fwa".into(),
            features,
        })
        .await
        .map_err(|error| (axum::http::StatusCode::BAD_GATEWAY, error.to_string()))?;
    let decision = fwa_scoring::aggregate(&rule_matches, &model_score);
    let audit_id = AuditEventId::new();

    Ok(Json(ScoreClaimResponse {
        run_id: run_id.to_string(),
        audit_id: audit_id.to_string(),
        claim_id: context.claim.external_claim_id,
        risk_score: decision.risk_score.value(),
        rag: decision.rag,
        recommended_action: decision.recommended_action,
        top_reasons: decision.top_reasons,
    }))
}

fn demo_context(claim_id: Option<String>, payload: Option<FullClaimPayload>) -> ClaimContext {
    let external_claim_id = claim_id
        .or_else(|| payload.as_ref().map(|claim| claim.external_claim_id.clone()))
        .unwrap_or_else(|| "CLM-DEMO".into());
    let amount = payload
        .as_ref()
        .map(|claim| claim.claim_amount)
        .unwrap_or_else(|| Decimal::new(8000, 0));
    let currency = payload
        .as_ref()
        .map(|claim| claim.currency.clone())
        .unwrap_or_else(|| "CNY".into());
    let member_id = MemberId::from_external("MBR-DEMO");
    let policy_id = PolicyId::from_external("POL-DEMO");
    let provider_id = ProviderId::from_external("PRV-DEMO");

    ClaimContext {
        claim: Claim {
            id: ClaimId::from_external(external_claim_id.clone()),
            external_claim_id,
            member_id: member_id.clone(),
            policy_id: policy_id.clone(),
            provider_id: provider_id.clone(),
            diagnosis_code: "J10".into(),
            service_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
            amount: Money::new(amount, currency),
        },
        items: vec![],
        member: Member {
            id: member_id.clone(),
            external_member_id: "MBR-DEMO".into(),
            dob: None,
            gender: None,
        },
        policy: Policy {
            id: policy_id,
            external_policy_id: "POL-DEMO".into(),
            member_id,
            product_code: "MED".into(),
            coverage_start_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            coverage_end_date: chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            coverage_limit: Money::new(Decimal::new(10000, 0), "CNY"),
        },
        provider: Provider {
            id: provider_id,
            external_provider_id: "PRV-DEMO".into(),
            name: "Demo Hospital".into(),
            provider_type: "hospital".into(),
            region: "SH".into(),
            risk_tier: ProviderRiskTier::Medium,
        },
    }
}

fn demo_rules() -> Vec<Rule> {
    vec![Rule {
        rule_id: "rule_early_claim".into(),
        version: 1,
        name: "Early claim".into(),
        conditions: vec![Condition {
            field: "days_since_policy_start".into(),
            operator: "<=".into(),
            value: serde_json::json!(7),
        }],
        action: RuleAction {
            score: 25,
            alert_code: "EARLY_CLAIM".into(),
            recommended_action: RecommendedAction::ManualReview,
            reason: "保单生效后 7 天内发生理赔".into(),
        },
    }]
}
```

- [ ] **Step 4: Wire modules and main**

Create `apps/api-server/src/routes/mod.rs`:

```rust
pub mod claims;
pub mod health;
```

Replace `apps/api-server/src/main.rs`:

```rust
use api_server::app::build_app;
use api_server::config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let app = build_app(AppConfig::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("api-server listening on 127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 5: Add integration test**

Create `apps/api-server/tests/claims_score.rs`:

```rust
use api_server::app::build_app;
use api_server::config::AppConfig;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn scores_full_payload_with_api_key() {
    let app = build_app(AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .header("x-api-key", "dev-secret")
        .body(Body::from(
            r#"{
              "source_system": "tpa-demo",
              "claim": {
                "external_claim_id": "CLM-0287",
                "claim_amount": "8000",
                "currency": "CNY"
              }
            }"#,
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn rejects_missing_api_key() {
    let app = build_app(AppConfig {
        api_key: "dev-secret".into(),
        source_system: "tpa-demo".into(),
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/claims/score")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"source_system":"tpa-demo","claim_id":"CLM-1"}"#))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 6: Run tests**

```bash
cargo fmt --all
cargo test -p api-server
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/api-server crates/fwa-*
git commit -m "feat: add scoring API vertical slice"
```

---

### Task 12: Replace API In-Memory Persistence with SQLx Repositories

**Files:**
- Modify: `apps/api-server/Cargo.toml`
- Create: `apps/api-server/src/repository.rs`
- Modify: `apps/api-server/src/app.rs`
- Modify: `apps/api-server/src/routes/claims.rs`
- Modify: `apps/api-server/src/config.rs`

- [ ] **Step 1: Add SQLx dependency**

Ensure `apps/api-server/Cargo.toml` includes:

```toml
sqlx.workspace = true
```

- [ ] **Step 2: Extend config**

Modify `apps/api-server/src/config.rs`:

```rust
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub source_system: String,
    pub database_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("FWA_API_KEY").unwrap_or_else(|_| "dev-secret".into()),
            source_system: std::env::var("FWA_SOURCE_SYSTEM").unwrap_or_else(|_| "tpa-demo".into()),
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:postgres@localhost:5432/fwa".into()
            }),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
```

- [ ] **Step 3: Add repository API**

Create `apps/api-server/src/repository.rs`:

```rust
use sqlx::PgPool;

#[derive(Clone)]
pub struct Repository {
    pub pool: PgPool,
}

impl Repository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_scoring_run(&self, run_id: &str, source_system: &str, actor_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO scoring_runs (run_id, source_system, actor_id, status) VALUES ($1, $2, $3, 'succeeded')",
        )
        .bind(run_id)
        .bind(source_system)
        .bind(actor_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
```

- [ ] **Step 4: Wire pool into app state**

Modify `apps/api-server/src/app.rs` so `AppState` includes `Repository` and `build_app` accepts it:

```rust
use crate::repository::Repository;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub scorer: Arc<HeuristicModelScorer>,
    pub repository: Repository,
}

pub fn build_app(config: AppConfig, repository: Repository) -> Router {
    let state = AppState {
        config,
        scorer: Arc::new(HeuristicModelScorer),
        repository,
    };
    Router::new()
        .route("/api/v1/health", get(health::health))
        .route("/api/v1/claims/score", post(claims::score_claim))
        .with_state(state)
}
```

- [ ] **Step 5: Update main**

Modify `apps/api-server/src/main.rs`:

```rust
mod app;
mod config;
mod repository;
mod routes;

use app::build_app;
use config::AppConfig;
use repository::Repository;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let config = AppConfig::from_env();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    let app = build_app(config, Repository::new(pool));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("api-server listening on 127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}
```

- [ ] **Step 6: Persist scoring run from route**

In `apps/api-server/src/routes/claims.rs`, after `run_id` is created and before returning, call:

```rust
state
    .repository
    .insert_scoring_run(run_id.as_str(), &request.source_system, "tpa-demo")
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("SCORING_PERSISTENCE_FAILED: {error}"),
        )
    })?;
```

- [ ] **Step 7: Run tests**

```bash
cargo fmt --all
cargo test -p api-server
```

Expected: PASS for compile/unit tests. Database integration requires Docker Compose in Task 13.

- [ ] **Step 8: Commit**

```bash
git add apps/api-server
git commit -m "feat: add SQLx repository boundary"
```

---

### Task 12B: Persist Full Scoring Lineage

**Files:**
- Modify: `apps/api-server/src/repository.rs`
- Modify: `apps/api-server/src/routes/claims.rs`

- [ ] **Step 1: Extend repository with runtime record methods**

Add these methods inside `impl Repository` in `apps/api-server/src/repository.rs`:

```rust
pub async fn insert_feature_value(
    &self,
    run_id: &str,
    feature_name: &str,
    feature_version: i32,
    value_json: serde_json::Value,
    evidence_json: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO feature_values (run_id, feature_name, feature_version, value_json, evidence_json)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(run_id)
    .bind(feature_name)
    .bind(feature_version)
    .bind(value_json)
    .bind(evidence_json)
    .execute(&self.pool)
    .await?;
    Ok(())
}

pub async fn insert_rule_run(
    &self,
    run_id: &str,
    matched: bool,
    score_contribution: i32,
    alert_code: Option<&str>,
    reason: Option<&str>,
    evidence_json: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO rule_runs (run_id, matched, score_contribution, alert_code, reason, evidence_json)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(run_id)
    .bind(matched)
    .bind(score_contribution)
    .bind(alert_code)
    .bind(reason)
    .bind(evidence_json)
    .execute(&self.pool)
    .await?;
    Ok(())
}

pub async fn insert_model_score(
    &self,
    run_id: &str,
    model_key: &str,
    runtime_kind: &str,
    execution_provider: &str,
    score: i32,
    label: &str,
    explanation_json: serde_json::Value,
    latency_ms: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO model_scores
         (run_id, model_key, runtime_kind, execution_provider, score, label, explanation_json, latency_ms)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(run_id)
    .bind(model_key)
    .bind(runtime_kind)
    .bind(execution_provider)
    .bind(score)
    .bind(label)
    .bind(explanation_json)
    .bind(latency_ms)
    .execute(&self.pool)
    .await?;
    Ok(())
}

pub async fn insert_audit_event(
    &self,
    audit_id: &str,
    run_id: &str,
    actor_id: &str,
    actor_role: &str,
    source_system: &str,
    event_type: &str,
    event_status: &str,
    summary: &str,
    payload: serde_json::Value,
    evidence_refs: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_events
         (audit_id, run_id, actor_id, actor_role, source_system, event_type, event_status, summary, payload, evidence_refs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(audit_id)
    .bind(run_id)
    .bind(actor_id)
    .bind(actor_role)
    .bind(source_system)
    .bind(event_type)
    .bind(event_status)
    .bind(summary)
    .bind(payload)
    .bind(evidence_refs)
    .execute(&self.pool)
    .await?;
    Ok(())
}
```

- [ ] **Step 2: Persist feature values from the route**

In `apps/api-server/src/routes/claims.rs`, move `let run_id = ScoringRunId::new();` so it appears immediately after `let context = demo_context(request.claim_id, request.claim);`. Then insert feature persistence immediately after `let features = calculate_features(&context);`:

```rust
for feature in features.values() {
    state
        .repository
        .insert_feature_value(
            run_id.as_str(),
            &feature.name,
            feature.version as i32,
            feature.value.clone(),
            serde_json::to_value(&feature.evidence_refs).unwrap_or_else(|_| serde_json::json!([])),
        )
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("FEATURE_PERSISTENCE_FAILED: {error}"),
            )
        })?;
}
```

- [ ] **Step 3: Persist rule runs, model score, and audit event**

In `apps/api-server/src/routes/claims.rs`, after rule evaluation and model scoring, insert:

```rust
for rule_match in &rule_matches {
    state
        .repository
        .insert_rule_run(
            run_id.as_str(),
            true,
            rule_match.score_contribution as i32,
            Some(&rule_match.alert_code),
            Some(&rule_match.reason),
            serde_json::json!([]),
        )
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("RULE_RUN_PERSISTENCE_FAILED: {error}"),
            )
        })?;
}

state
    .repository
    .insert_model_score(
        run_id.as_str(),
        &model_score.model_key,
        &model_score.runtime_kind,
        &model_score.execution_provider,
        model_score.score as i32,
        &model_score.label,
        serde_json::to_value(&model_score.explanations).unwrap_or_else(|_| serde_json::json!([])),
        model_score.latency_ms as i32,
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("MODEL_SCORE_PERSISTENCE_FAILED: {error}"),
        )
    })?;

let audit_id = AuditEventId::new();
state
    .repository
    .insert_audit_event(
        audit_id.as_str(),
        run_id.as_str(),
        "tpa-demo",
        "tpa_system",
        &request.source_system,
        "scoring.completed",
        "succeeded",
        "FWA scoring completed",
        serde_json::json!({ "risk_score": decision.risk_score.value() }),
        serde_json::json!([]),
    )
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("AUDIT_PERSISTENCE_FAILED: {error}"),
        )
    })?;
```

- [ ] **Step 4: Run tests**

```bash
cargo fmt --all
cargo test -p api-server
```

Expected: PASS for compile/unit tests. End-to-end persistence is verified after Docker Compose is available.

- [ ] **Step 5: Commit**

```bash
git add apps/api-server
git commit -m "feat: persist scoring lineage records"
```

---

### Task 12C: Persist and Load Structured Claim Context

**Files:**
- Modify: `apps/api-server/src/repository.rs`
- Modify: `apps/api-server/src/routes/claims.rs`

- [ ] **Step 1: Add repository methods for full payload upsert and claim loading**

Add these method signatures and implementations to `apps/api-server/src/repository.rs`. Keep the first implementation focused on the MVP demo payload fields; extend the payload DTO in the same task only for fields required by `ClaimContext`.

```rust
use fwa_core::ClaimContext;

pub async fn upsert_claim_context(
    &self,
    context: &ClaimContext,
    raw_payload: serde_json::Value,
) -> Result<(), sqlx::Error> {
    let mut tx = self.pool.begin().await?;

    let member_row: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO members (external_member_id)
         VALUES ($1)
         ON CONFLICT (external_member_id) DO UPDATE SET updated_at = now()
         RETURNING id",
    )
    .bind(&context.member.external_member_id)
    .fetch_one(&mut *tx)
    .await?;

    let policy_row: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO policies
         (external_policy_id, member_id, product_code, coverage_start_date, coverage_end_date, coverage_limit_amount, currency)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (external_policy_id) DO UPDATE SET updated_at = now()
         RETURNING id",
    )
    .bind(&context.policy.external_policy_id)
    .bind(member_row.0)
    .bind(&context.policy.product_code)
    .bind(context.policy.coverage_start_date)
    .bind(context.policy.coverage_end_date)
    .bind(context.policy.coverage_limit.amount)
    .bind(&context.policy.coverage_limit.currency)
    .fetch_one(&mut *tx)
    .await?;

    let provider_row: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO providers (external_provider_id, name, provider_type, region, risk_tier)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (external_provider_id) DO UPDATE SET updated_at = now()
         RETURNING id",
    )
    .bind(&context.provider.external_provider_id)
    .bind(&context.provider.name)
    .bind(&context.provider.provider_type)
    .bind(&context.provider.region)
    .bind(format!("{:?}", context.provider.risk_tier))
    .fetch_one(&mut *tx)
    .await?;

    let claim_row: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO claims
         (external_claim_id, member_id, policy_id, provider_id, claim_type, diagnosis_code, service_date, claim_amount, currency, status, raw_payload)
         VALUES ($1, $2, $3, $4, 'medical', $5, $6, $7, $8, 'submitted', $9)
         ON CONFLICT (external_claim_id) DO UPDATE SET updated_at = now(), raw_payload = EXCLUDED.raw_payload
         RETURNING id",
    )
    .bind(&context.claim.external_claim_id)
    .bind(member_row.0)
    .bind(policy_row.0)
    .bind(provider_row.0)
    .bind(&context.claim.diagnosis_code)
    .bind(context.claim.service_date)
    .bind(context.claim.amount.amount)
    .bind(&context.claim.amount.currency)
    .bind(raw_payload)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM claim_items WHERE claim_id = $1")
        .bind(claim_row.0)
        .execute(&mut *tx)
        .await?;

    for item in &context.items {
        sqlx::query(
            "INSERT INTO claim_items
             (claim_id, item_code, item_type, description, quantity, unit_amount, total_amount, currency)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(claim_row.0)
        .bind(&item.item_code)
        .bind(&item.item_type)
        .bind(&item.description)
        .bind(item.quantity as i32)
        .bind(item.total_amount.amount)
        .bind(item.total_amount.amount)
        .bind(&item.total_amount.currency)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn load_claim_context(&self, external_claim_id: &str) -> Result<Option<ClaimContext>, sqlx::Error> {
    let raw_payload: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT raw_payload FROM claims WHERE external_claim_id = $1")
            .bind(external_claim_id)
            .fetch_optional(&self.pool)
            .await?;

    let Some((value,)) = raw_payload else {
        return Ok(None);
    };

    let context = serde_json::from_value(value).map_err(|error| {
        sqlx::Error::Decode(Box::new(error))
    })?;
    Ok(Some(context))
}
```

- [ ] **Step 2: Store full payload requests**

In `apps/api-server/src/routes/claims.rs`, after building `context` for full payload mode and before feature calculation, call:

```rust
state
    .repository
    .upsert_claim_context(&context, serde_json::to_value(&context).unwrap_or_else(|_| serde_json::json!({})))
    .await
    .map_err(|error| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("CLAIM_PERSISTENCE_FAILED: {error}"),
        )
    })?;
```

- [ ] **Step 3: Load existing claim requests**

Replace the `claim_id` branch that currently calls `demo_context` with:

```rust
let context = if let Some(claim_id) = request.claim_id.clone() {
    state
        .repository
        .load_claim_context(&claim_id)
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("CLAIM_LOAD_FAILED: {error}"),
            )
        })?
        .ok_or_else(|| (axum::http::StatusCode::NOT_FOUND, "CLAIM_NOT_FOUND".to_string()))?
} else {
    let context = demo_context(None, request.claim);
    state
        .repository
        .upsert_claim_context(&context, serde_json::to_value(&context).unwrap_or_else(|_| serde_json::json!({})))
        .await
        .map_err(|error| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("CLAIM_PERSISTENCE_FAILED: {error}"),
            )
        })?;
    context
};
```

- [ ] **Step 4: Run tests**

```bash
cargo fmt --all
cargo test -p api-server
```

Expected: PASS for compile/unit tests. A follow-up end-to-end run against Postgres must verify both full payload and `claim_id` modes.

- [ ] **Step 5: Commit**

```bash
git add apps/api-server
git commit -m "feat: persist and load claim context"
```

---

### Task 13: Add Docker Compose for Local Runtime

**Files:**
- Create: `infra/docker-compose.yml`
- Modify: `README.md`

- [ ] **Step 1: Add Docker Compose**

Create `infra/docker-compose.yml`:

```yaml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: fwa
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres -d fwa"]
      interval: 5s
      timeout: 5s
      retries: 10
    volumes:
      - fwa_postgres_data:/var/lib/postgresql/data

  ml-service:
    build:
      context: ../apps/ml-service
    ports:
      - "8001:8001"

volumes:
  fwa_postgres_data:
```

Create `apps/ml-service/Dockerfile`:

```dockerfile
FROM python:3.12-slim

WORKDIR /app
COPY pyproject.toml ./
COPY app ./app
RUN pip install --no-cache-dir .

EXPOSE 8001
CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8001"]
```

- [ ] **Step 2: Update README**

Add:

```markdown
## Local Development

Start dependencies:

```bash
docker compose -f infra/docker-compose.yml up postgres ml-service
```

Run API server:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/fwa FWA_API_KEY=dev-secret cargo run -p api-server
```

Run tests:

```bash
cargo test --workspace
cd apps/ml-service && pytest
```
```

- [ ] **Step 3: Run compose config check**

```bash
docker compose -f infra/docker-compose.yml config
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add infra/docker-compose.yml apps/ml-service/Dockerfile README.md
git commit -m "chore: add local runtime compose"
```

---

### Task 14: Add Operations Studio Skeleton

**Files:**
- Create: `apps/web-console/package.json`
- Create: `apps/web-console/index.html`
- Create: `apps/web-console/src/main.tsx`
- Create: `apps/web-console/src/App.tsx`
- Create: `apps/web-console/src/api.ts`
- Create: `apps/web-console/src/pages/RuntimeScoring.tsx`
- Create: `apps/web-console/src/pages/PlannedModulePage.tsx`
- Create: `apps/web-console/src/styles.css`

- [ ] **Step 1: Create package config**

Create `apps/web-console/package.json`:

```json
{
  "name": "fwa-web-console",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite --host 127.0.0.1 --port 5173",
    "build": "tsc && vite build",
    "lint": "tsc --noEmit",
    "test": "vitest run"
  },
  "dependencies": {
    "@vitejs/plugin-react": "^4.3.4",
    "vite": "^6.0.0",
    "typescript": "^5.7.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "@tanstack/react-query": "^5.62.0"
  },
  "devDependencies": {
    "vitest": "^2.1.8"
  }
}
```

- [ ] **Step 2: Create app files**

Create `apps/web-console/index.html`:

```html
<div id="root"></div>
<script type="module" src="/src/main.tsx"></script>
```

Create `apps/web-console/src/main.tsx`:

```tsx
import React from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { App } from "./App";
import "./styles.css";

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
```

Create `apps/web-console/src/api.ts`:

```ts
export async function scoreClaim(payload: unknown, apiKey: string) {
  const response = await fetch("/api/v1/claims/score", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-api-key": apiKey,
    },
    body: JSON.stringify(payload),
  });

  const body = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(typeof body === "string" ? body : JSON.stringify(body));
  }
  return body;
}
```

Create `apps/web-console/src/pages/PlannedModulePage.tsx`:

```tsx
export function PlannedModulePage({ title }: { title: string }) {
  return (
    <section className="panel">
      <h2>{title}</h2>
      <p>This module is planned for a later phase. No production API is available yet.</p>
    </section>
  );
}
```

Create `apps/web-console/src/pages/RuntimeScoring.tsx`:

```tsx
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { scoreClaim } from "../api";

const defaultPayload = JSON.stringify(
  {
    source_system: "tpa-demo",
    claim: {
      external_claim_id: "CLM-0287",
      claim_amount: "8000",
      currency: "CNY"
    }
  },
  null,
  2,
);

export function RuntimeScoring() {
  const [apiKey, setApiKey] = useState("dev-secret");
  const [payload, setPayload] = useState(defaultPayload);
  const mutation = useMutation({
    mutationFn: () => scoreClaim(JSON.parse(payload), apiKey),
  });

  return (
    <section className="runtime">
      <div className="panel">
        <h2>Runtime Scoring</h2>
        <label>
          API Key
          <input value={apiKey} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        <label>
          Claim Request JSON
          <textarea value={payload} onChange={(event) => setPayload(event.target.value)} />
        </label>
        <button onClick={() => mutation.mutate()} disabled={mutation.isPending}>
          Score Claim
        </button>
      </div>
      <div className="panel">
        <h2>Result</h2>
        {mutation.error ? <pre className="error">{String(mutation.error.message)}</pre> : null}
        {mutation.data ? <pre>{JSON.stringify(mutation.data, null, 2)}</pre> : null}
      </div>
    </section>
  );
}
```

Create `apps/web-console/src/App.tsx`:

```tsx
import { RuntimeScoring } from "./pages/RuntimeScoring";
import { PlannedModulePage } from "./pages/PlannedModulePage";

const modules = [
  "Dashboard",
  "Runtime Scoring",
  "Rules",
  "Models",
  "Factor Factory",
  "Knowledge Base",
  "QA Review",
  "Governance",
];

export function App() {
  const active = "Runtime Scoring";
  return (
    <div className="app">
      <aside>
        <h1>FWA Studio</h1>
        {modules.map((module) => (
          <button className={module === active ? "active" : ""} key={module}>
            {module}
          </button>
        ))}
      </aside>
      <main>
        {active === "Runtime Scoring" ? <RuntimeScoring /> : <PlannedModulePage title={active} />}
      </main>
    </div>
  );
}
```

Create `apps/web-console/src/styles.css`:

```css
body {
  margin: 0;
  font-family: Inter, system-ui, sans-serif;
  background: #f6f7f9;
  color: #171717;
}

.app {
  display: grid;
  grid-template-columns: 240px 1fr;
  min-height: 100vh;
}

aside {
  background: #101820;
  color: white;
  padding: 20px;
}

aside button {
  display: block;
  width: 100%;
  margin: 6px 0;
  padding: 10px;
  text-align: left;
  background: transparent;
  color: white;
  border: 1px solid transparent;
}

aside button.active {
  border-color: #8fb3ff;
  background: #1f2f3f;
}

main {
  padding: 24px;
}

.runtime {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.panel {
  background: white;
  border: 1px solid #d8dde4;
  border-radius: 8px;
  padding: 18px;
}

label {
  display: grid;
  gap: 6px;
  margin: 12px 0;
}

input,
textarea {
  font: inherit;
  padding: 10px;
  border: 1px solid #c7ccd4;
  border-radius: 6px;
}

textarea {
  min-height: 320px;
}

pre {
  white-space: pre-wrap;
  overflow: auto;
}

.error {
  color: #a40000;
}
```

- [ ] **Step 3: Run frontend checks**

```bash
cd apps/web-console
npm install
npm run build
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/web-console
git commit -m "feat: add Operations Studio runtime scoring UI"
```

---

### Task 15: Upgrade CI for Implemented Stacks

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `scripts/ci/check_repo.sh`

- [ ] **Step 1: Add required files to health check**

Add these paths to `required_files` in `scripts/ci/check_repo.sh`:

```bash
  "Cargo.toml"
  "apps/ml-service/pyproject.toml"
  "apps/web-console/package.json"
  "migrations/0001_initial.sql"
```

- [ ] **Step 2: Replace CI workflow**

Replace `.github/workflows/ci.yml` with:

```yaml
name: CI

on:
  push:
    branches:
      - main
      - develop
      - "feature/**"
      - "release/**"
      - "hotfix/**"
  pull_request:
    branches:
      - main
      - develop
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  repository-health:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: bash scripts/ci/check_repo.sh

  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace

  python:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: apps/ml-service
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - run: python -m pip install -e ".[dev]"
      - run: pytest

  frontend:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: apps/web-console
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: apps/web-console/package-lock.json
      - run: npm ci
      - run: npm run lint
      - run: npm test
      - run: npm run build
```

- [ ] **Step 3: Run local checks**

```bash
bash scripts/ci/check_repo.sh
cargo fmt --all -- --check
cargo test --workspace
cd apps/ml-service && pytest
cd ../web-console && npm run build
```

Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml scripts/ci/check_repo.sh
git commit -m "ci: check Rust Python and frontend stacks"
```

---

### Task 16: Final Verification and Push

**Files:**
- Modify only if verification exposes a defect.

- [ ] **Step 1: Run full local verification**

```bash
git status --short --branch
bash scripts/ci/check_repo.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/ml-service && pytest
cd ../web-console && npm run build
```

Expected: all PASS and worktree clean after committed changes.

- [ ] **Step 2: Push branch**

```bash
git push origin main
```

- [ ] **Step 3: Check GitHub Actions**

```bash
gh run list --repo proerror77/nwfwa --limit 5
```

Expected: latest CI run is `completed success`.

- [ ] **Step 4: Report evidence**

Final response must include:

- commit range
- local commands run
- GitHub Actions latest run status
- any known gaps against the MVP spec

---

## Self-Review

Spec coverage:

- Workspace and Rust crate boundaries: Tasks 1-8.
- Feature/rule/model/scoring/audit core: Tasks 2-8.
- Database schema: Task 9.
- Python ML service: Task 10.
- API scoring path, scoring lineage persistence, and claim context persistence: Tasks 11, 12, 12B, and 12C.
- Local runtime: Task 13.
- Operations Studio: Task 14.
- CI: Task 15.
- Final verification: Task 16.

Known scoped gap:

- End-to-end Postgres verification depends on Docker Compose being available in the execution environment. If Docker is unavailable, the worker must report that limitation and provide the last passing compile/unit-test evidence instead of claiming database verification.

No deferred modules are implemented as fake production behavior. Planned frontend module pages explicitly state they are not backed by production APIs yet.
