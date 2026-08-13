use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::State;
use uuid::Uuid;

use crate::{database::Database, state::AppState};

const MEMORY_POLICY_VERSION: &str = "evolving-memory-v1";
const DAY_MS: i64 = 86_400_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub id: String,
    pub kind: String,
    pub content: String,
    pub importance: f64,
    pub confidence: f64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_accessed_at: Option<i64>,
    pub access_count: i64,
    pub valid_from: Option<i64>,
    pub valid_to: Option<i64>,
    pub source_type: Option<String>,
    pub source_ref: Option<String>,
    pub pinned: bool,
}

pub struct NewMemory {
    pub id: String,
    pub kind: String,
    pub content: String,
    pub importance: f64,
    pub confidence: f64,
    pub created_at: i64,
    pub valid_from: Option<i64>,
    pub valid_to: Option<i64>,
    pub source_type: String,
    pub source_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRelation {
    pub source_id: String,
    pub target_id: String,
    pub relation: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryOperation {
    pub id: String,
    pub memory_id: Option<String>,
    pub operation: String,
    pub timestamp: i64,
    pub source_event_id: Option<String>,
    pub reason_label: Option<String>,
    pub related_memory_ids_json: Option<String>,
    pub detail_json: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCandidate {
    pub content: String,
    pub source_ref: Option<String>,
    pub source_type: Option<String>,
    pub occurred_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryLifecycleDecision {
    pub operation: String,
    pub memory_type: String,
    pub content: String,
    pub metadata: Value,
    pub related_memory_ids: Vec<String>,
    pub reason_label: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryWriteResult {
    pub decision: String,
    pub reason: String,
    pub memory: Option<Memory>,
    pub lifecycle: MemoryLifecycleDecision,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryFilter {
    pub kind: Option<String>,
    pub status: Option<String>,
    pub query: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatusCounts {
    pub active: i64,
    pub outdated: i64,
    pub archived: i64,
    pub merged: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryViewerSnapshot {
    pub memories: Vec<Memory>,
    pub relations: Vec<MemoryRelation>,
    pub operations: Vec<MemoryOperation>,
    pub counts: MemoryStatusCounts,
    pub policy_version: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDetail {
    pub memory: Memory,
    pub relations: Vec<MemoryRelation>,
    pub operations: Vec<MemoryOperation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMemoryEdit {
    pub content: String,
    pub kind: Option<String>,
    pub importance: Option<f64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDeleteResult {
    pub deleted_id: String,
    pub relations_deleted: usize,
    pub operations_deleted: usize,
}

pub fn timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn normalize(content: &str) -> String {
    content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

fn worth_remembering(content: &str) -> bool {
    let normalized = normalize(content);
    let meaningful_chars = normalized
        .chars()
        .filter(|character| character.is_alphanumeric())
        .count();
    if !(4..=2_000).contains(&meaningful_chars)
        || normalized.ends_with('?')
        || normalized.ends_with('？')
    {
        return false;
    }
    let trivial = [
        "hello",
        "hi",
        "hey",
        "thanks",
        "thank you",
        "good morning",
        "good night",
        "你好",
        "谢谢",
        "早上好",
        "晚安",
        "哈哈哈",
    ];
    !trivial.contains(&normalized.as_str())
        && !normalized.contains("user greeted")
        && !normalized.contains("用户打了招呼")
}

fn classify(content: &str) -> &'static str {
    let lower = content.to_lowercase();
    if contains_any(
        &lower,
        &[
            "remi",
            "desktop pet",
            "our relationship",
            "shared",
            "桌宠",
            "我们之间",
            "给你取名",
        ],
    ) {
        return "relationship";
    }
    if contains_any(
        &lower,
        &[
            "today",
            "yesterday",
            "tomorrow",
            "last week",
            "recently",
            "今天",
            "昨天",
            "明天",
            "上周",
            "最近",
            "正在",
            "准备",
        ],
    ) {
        "episodic"
    } else {
        "semantic"
    }
}

fn source_type(candidate: &MemoryCandidate) -> String {
    let candidate = candidate.source_type.as_deref().unwrap_or("agent_inferred");
    match candidate {
        "user_explicit" | "agent_inferred" | "conversation" | "heartbeat" | "reflection"
        | "system" => candidate.to_string(),
        _ => "agent_inferred".to_string(),
    }
}

fn temporal_validity(content: &str, occurred_at: i64) -> (Option<i64>, Option<i64>) {
    let lower = content.to_lowercase();
    if contains_any(&lower, &["tomorrow", "明天"]) {
        return (Some(occurred_at), Some(occurred_at + 2 * DAY_MS));
    }
    if contains_any(&lower, &["today", "今天", "tonight", "今晚"]) {
        return (Some(occurred_at), Some(occurred_at + DAY_MS));
    }
    if contains_any(&lower, &["this week", "本周", "这周", "next week", "下周"]) {
        return (Some(occurred_at), Some(occurred_at + 14 * DAY_MS));
    }
    (None, None)
}

fn memory_slot(content: &str) -> Option<&'static str> {
    let lower = content.to_lowercase();
    if contains_any(
        &lower,
        &["coffee", "tea", "咖啡", "茶", "espresso", "latte"],
    ) && contains_any(
        &lower,
        &[
            "drink", "prefer", "like", "favorite", "喝", "喜欢", "偏好", "最爱",
        ],
    ) {
        return Some("preference:beverage");
    }
    if contains_any(&lower, &["cat", "猫"]) && contains_any(&lower, &["name", "叫", "名字"]) {
        return Some("fact:cat_name");
    }
    if contains_any(&lower, &["dog", "狗"]) && contains_any(&lower, &["name", "叫", "名字"]) {
        return Some("fact:dog_name");
    }
    if contains_any(&lower, &["live in", "lives in", "住在", "居住"]) {
        return Some("fact:residence");
    }
    if contains_any(&lower, &["work as", "works as", "职业", "工作是"]) {
        return Some("fact:occupation");
    }
    if contains_any(
        &lower,
        &["prefer", "favorite", "likes", "偏好", "喜欢", "最爱"],
    ) {
        return Some("preference:general");
    }
    None
}

fn has_change_signal(content: &str) -> bool {
    contains_any(
        &content.to_lowercase(),
        &[
            "no longer",
            "don't",
            "doesn't",
            "not anymore",
            "now prefer",
            "changed",
            "不再",
            "不喜欢",
            "改成",
            "现在更",
            "不要",
        ],
    )
}

fn terms(text: &str) -> HashSet<String> {
    let lower = text.to_lowercase();
    let mut result: HashSet<String> = lower
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(ToString::to_string)
        .collect();
    let compact: Vec<char> = lower
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect();
    for pair in compact.windows(2) {
        result.insert(pair.iter().collect());
    }
    result
}

fn relevance(query: &str, content: &str) -> f64 {
    let query_terms = terms(query);
    let content_terms = terms(content);
    if query_terms.is_empty() || content_terms.is_empty() {
        return 0.0;
    }
    let overlap = query_terms.intersection(&content_terms).count() as f64;
    overlap / query_terms.len().max(1) as f64
}

fn operation(
    memory_id: Option<String>,
    operation: &str,
    timestamp: i64,
    source_event_id: Option<String>,
    reason_label: &str,
    related_memory_ids: &[String],
    detail: Value,
) -> MemoryOperation {
    MemoryOperation {
        id: Uuid::new_v4().to_string(),
        memory_id,
        operation: operation.to_string(),
        timestamp,
        source_event_id,
        reason_label: Some(reason_label.to_string()),
        related_memory_ids_json: Some(json!(related_memory_ids).to_string()),
        detail_json: Some(detail.to_string()),
    }
}

// Keeping decision fields visible here makes the persisted research contract explicit.
#[allow(clippy::too_many_arguments)]
fn decision(
    operation: &str,
    memory_type: &str,
    content: &str,
    source: &str,
    valid_from: Option<i64>,
    valid_to: Option<i64>,
    related_memory_ids: Vec<String>,
    reason_label: &str,
) -> MemoryLifecycleDecision {
    MemoryLifecycleDecision {
        operation: operation.to_string(),
        memory_type: memory_type.to_string(),
        content: content.to_string(),
        metadata: json!({
            "sourceType": source,
            "validFrom": valid_from,
            "validTo": valid_to,
            "policyVersion": MEMORY_POLICY_VERSION
        }),
        related_memory_ids,
        reason_label: reason_label.to_string(),
    }
}

pub fn write_candidate(
    database: &Database,
    candidate: MemoryCandidate,
) -> Result<MemoryWriteResult, String> {
    let content = candidate.content.trim().to_string();
    let now = candidate.occurred_at.unwrap_or_else(timestamp_ms);
    let kind = classify(&content).to_string();
    let source = source_type(&candidate);
    let (valid_from, valid_to) = temporal_validity(&content, now);

    if !worth_remembering(&content) {
        let lifecycle = decision(
            "IGNORE",
            &kind,
            &content,
            &source,
            valid_from,
            valid_to,
            Vec::new(),
            "low_value",
        );
        database
            .insert_memory_operation(&operation(
                None,
                "IGNORE",
                now,
                candidate.source_ref,
                "low_value",
                &[],
                lifecycle.metadata.clone(),
            ))
            .map_err(|error| error.to_string())?;
        return Ok(MemoryWriteResult {
            decision: "IGNORE".to_string(),
            reason: "Candidate is not useful for future interactions".to_string(),
            memory: None,
            lifecycle,
        });
    }

    expire(database, now)?;
    let existing = database
        .list_active_memories()
        .map_err(|error| error.to_string())?;
    if let Some(duplicate) = existing
        .iter()
        .find(|memory| normalize(&memory.content) == normalize(&content))
    {
        let related = vec![duplicate.id.clone()];
        let lifecycle = decision(
            "IGNORE",
            &kind,
            &content,
            &source,
            valid_from,
            valid_to,
            related.clone(),
            "duplicate",
        );
        database
            .insert_memory_operation(&operation(
                Some(duplicate.id.clone()),
                "IGNORE",
                now,
                candidate.source_ref,
                "duplicate",
                &related,
                lifecycle.metadata.clone(),
            ))
            .map_err(|error| error.to_string())?;
        return Ok(MemoryWriteResult {
            decision: "IGNORE".to_string(),
            reason: "An equivalent active memory already exists".to_string(),
            memory: Some(duplicate.clone()),
            lifecycle,
        });
    }

    let slot = memory_slot(&content);
    let related = existing
        .iter()
        .find(|memory| {
            slot.is_some() && memory_slot(&memory.content) == slot && memory.kind == kind
        })
        .cloned();
    let importance: f64 = match kind.as_str() {
        "relationship" => 0.75,
        "semantic" => 0.7,
        _ => 0.6,
    };

    if let Some(previous) = related {
        let previous_normalized = normalize(&previous.content);
        let enrichment = normalize(&content).contains(&previous_normalized)
            && !has_change_signal(&content)
            && !memory_slot(&content).is_some_and(|slot| slot.starts_with("preference:"));
        if enrichment {
            let memory = database
                .update_memory(
                    &previous.id,
                    &content,
                    &kind,
                    importance.max(previous.importance),
                    previous.confidence.max(0.9),
                    now,
                    valid_from.or(previous.valid_from),
                    valid_to.or(previous.valid_to),
                )
                .map_err(|error| error.to_string())?;
            let related_ids = vec![previous.id.clone()];
            let lifecycle = decision(
                "UPDATE",
                &kind,
                &content,
                &source,
                memory.valid_from,
                memory.valid_to,
                related_ids.clone(),
                "stable_fact_enrichment",
            );
            database
                .insert_memory_operation(&operation(
                    Some(memory.id.clone()),
                    "UPDATE",
                    now,
                    candidate.source_ref,
                    "stable_fact_enrichment",
                    &related_ids,
                    lifecycle.metadata.clone(),
                ))
                .map_err(|error| error.to_string())?;
            return Ok(MemoryWriteResult {
                decision: "UPDATE".to_string(),
                reason: "Existing stable memory was enriched".to_string(),
                memory: Some(memory),
                lifecycle,
            });
        }

        let memory = database
            .insert_memory(&NewMemory {
                id: Uuid::new_v4().to_string(),
                kind: kind.clone(),
                content: content.clone(),
                importance,
                confidence: 0.9,
                created_at: now,
                valid_from,
                valid_to,
                source_type: source.clone(),
                source_ref: candidate.source_ref.clone(),
            })
            .map_err(|error| error.to_string())?;
        database
            .set_memory_status(&previous.id, "outdated", now)
            .map_err(|error| error.to_string())?;
        database
            .insert_memory_relation(&MemoryRelation {
                source_id: memory.id.clone(),
                target_id: previous.id.clone(),
                relation: "supersedes".to_string(),
                created_at: now,
            })
            .map_err(|error| error.to_string())?;
        if has_change_signal(&content) {
            database
                .insert_memory_relation(&MemoryRelation {
                    source_id: memory.id.clone(),
                    target_id: previous.id.clone(),
                    relation: "contradicts".to_string(),
                    created_at: now,
                })
                .map_err(|error| error.to_string())?;
        }
        let related_ids = vec![previous.id];
        let lifecycle = decision(
            "SUPERSEDE",
            &kind,
            &content,
            &source,
            valid_from,
            valid_to,
            related_ids.clone(),
            "same_slot_new_value",
        );
        database
            .insert_memory_operation(&operation(
                Some(memory.id.clone()),
                "SUPERSEDE",
                now,
                candidate.source_ref,
                "same_slot_new_value",
                &related_ids,
                lifecycle.metadata.clone(),
            ))
            .map_err(|error| error.to_string())?;
        return Ok(MemoryWriteResult {
            decision: "SUPERSEDE".to_string(),
            reason: "New information replaced an active memory in the same slot".to_string(),
            memory: Some(memory),
            lifecycle,
        });
    }

    let memory = database
        .insert_memory(&NewMemory {
            id: Uuid::new_v4().to_string(),
            kind: kind.clone(),
            content: content.clone(),
            importance,
            confidence: 0.9,
            created_at: now,
            valid_from,
            valid_to,
            source_type: source.clone(),
            source_ref: candidate.source_ref.clone(),
        })
        .map_err(|error| error.to_string())?;
    let lifecycle = decision(
        "ADD",
        &kind,
        &content,
        &source,
        valid_from,
        valid_to,
        Vec::new(),
        "durable_candidate",
    );
    database
        .insert_memory_operation(&operation(
            Some(memory.id.clone()),
            "ADD",
            now,
            candidate.source_ref,
            "durable_candidate",
            &[],
            lifecycle.metadata.clone(),
        ))
        .map_err(|error| error.to_string())?;
    Ok(MemoryWriteResult {
        decision: "ADD".to_string(),
        reason: "Candidate passed selective memory writing".to_string(),
        memory: Some(memory),
        lifecycle,
    })
}

fn source_reliability(source: Option<&str>) -> f64 {
    match source {
        Some("user_explicit" | "user_correction") => 1.0,
        Some("conversation") => 0.85,
        Some("system") => 0.8,
        Some("reflection") => 0.7,
        Some("agent_inferred") => 0.6,
        Some("heartbeat") => 0.45,
        _ => 0.5,
    }
}

fn expire(database: &Database, now: i64) -> Result<(), String> {
    for id in database
        .expire_memories(now)
        .map_err(|error| error.to_string())?
    {
        database
            .insert_memory_operation(&operation(
                Some(id.clone()),
                "UPDATE",
                now,
                None,
                "validity_expired",
                &[id],
                json!({"status": "outdated", "policyVersion": MEMORY_POLICY_VERSION}),
            ))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn retrieve(
    database: &Database,
    query: &str,
    limit: usize,
    now: i64,
) -> Result<Vec<Memory>, String> {
    expire(database, now)?;
    let mut scored: Vec<(f64, Memory)> = database
        .list_active_memories()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|memory| {
            let age_days = (now - memory.updated_at).max(0) as f64 / DAY_MS as f64;
            let recency = 1.0 / (1.0 + age_days / 30.0);
            let score = 0.45 * relevance(query, &memory.content)
                + 0.15 * recency
                + 0.15 * memory.importance
                + 0.15 * memory.confidence
                + 0.1 * source_reliability(memory.source_type.as_deref());
            (score, memory)
        })
        .collect();
    scored.sort_by(|left, right| right.0.total_cmp(&left.0));
    let selected: Vec<Memory> = scored
        .into_iter()
        .take(limit.clamp(1, 8))
        .map(|(_, memory)| memory)
        .collect();
    let ids: Vec<String> = selected.iter().map(|memory| memory.id.clone()).collect();
    database
        .touch_memories(&ids, now)
        .map_err(|error| error.to_string())?;
    database
        .insert_memory_operation(&operation(
            None,
            "RETRIEVE",
            now,
            None,
            "ranked_active_only",
            &ids,
            json!({"query": query, "memoryIds": ids, "policyVersion": MEMORY_POLICY_VERSION}),
        ))
        .map_err(|error| error.to_string())?;
    Ok(selected)
}

pub fn consolidate(database: &Database, now: i64) -> Result<usize, String> {
    let active = database
        .list_active_memories()
        .map_err(|error| error.to_string())?;
    let mut canonical_by_content = std::collections::HashMap::<String, String>::new();
    let mut merged = 0;
    for memory in active.into_iter().rev() {
        let normalized = normalize(&memory.content);
        if let Some(canonical_id) = canonical_by_content.get(&normalized) {
            database
                .set_memory_status(&memory.id, "merged", now)
                .map_err(|error| error.to_string())?;
            database
                .insert_memory_relation(&MemoryRelation {
                    source_id: memory.id.clone(),
                    target_id: canonical_id.clone(),
                    relation: "merged_into".to_string(),
                    created_at: now,
                })
                .map_err(|error| error.to_string())?;
            database
                .insert_memory_operation(&operation(
                    Some(memory.id.clone()),
                    "CONSOLIDATE",
                    now,
                    None,
                    "exact_duplicate_merge",
                    std::slice::from_ref(canonical_id),
                    json!({"mergedInto": canonical_id, "policyVersion": MEMORY_POLICY_VERSION}),
                ))
                .map_err(|error| error.to_string())?;
            merged += 1;
        } else {
            canonical_by_content.insert(normalized, memory.id);
        }
    }
    Ok(merged)
}

pub fn archive(database: &Database, memory_id: &str, now: i64) -> Result<Memory, String> {
    let memory = database
        .get_memory(memory_id)
        .map_err(|error| error.to_string())?;
    if memory.status == "archived" {
        return Ok(memory);
    }
    database
        .set_memory_status(memory_id, "archived", now)
        .map_err(|error| error.to_string())?;
    database
        .insert_memory_operation(&operation(
            Some(memory_id.to_string()),
            "ARCHIVE",
            now,
            None,
            "user_archive",
            &[memory_id.to_string()],
            json!({"status": "archived", "policyVersion": MEMORY_POLICY_VERSION}),
        ))
        .map_err(|error| error.to_string())?;
    database
        .get_memory(memory_id)
        .map_err(|error| error.to_string())
}

fn validate_kind(kind: &str) -> Result<(), String> {
    if matches!(kind, "semantic" | "episodic" | "relationship") {
        Ok(())
    } else {
        Err(format!("Unsupported memory kind: {kind}"))
    }
}

fn validate_status(status: &str) -> Result<(), String> {
    if matches!(status, "active" | "outdated" | "archived" | "merged") {
        Ok(())
    } else {
        Err(format!("Unsupported memory status: {status}"))
    }
}

type NormalizedMemoryFilter = (Option<String>, Option<String>, Option<String>);

fn normalize_filter(filter: MemoryFilter) -> Result<NormalizedMemoryFilter, String> {
    if let Some(kind) = filter.kind.as_deref() {
        validate_kind(kind)?;
    }
    if let Some(status) = filter.status.as_deref() {
        validate_status(status)?;
    }
    let query = filter
        .query
        .map(|query| query.trim().to_string())
        .filter(|query| !query.is_empty());
    if query
        .as_ref()
        .is_some_and(|query| query.chars().count() > 500)
    {
        return Err("Memory search query must be 500 characters or fewer".to_string());
    }
    Ok((filter.kind, filter.status, query))
}

pub fn viewer_snapshot(
    database: &Database,
    filter: MemoryFilter,
    operation_limit: usize,
) -> Result<MemoryViewerSnapshot, String> {
    let (kind, status, query) = normalize_filter(filter)?;
    let (active, outdated, archived, merged) = database
        .memory_status_counts()
        .map_err(|error| error.to_string())?;
    Ok(MemoryViewerSnapshot {
        memories: database
            .search_memories(kind.as_deref(), status.as_deref(), query.as_deref())
            .map_err(|error| error.to_string())?,
        relations: database
            .list_memory_relations()
            .map_err(|error| error.to_string())?,
        operations: database
            .list_memory_operations(operation_limit)
            .map_err(|error| error.to_string())?,
        counts: MemoryStatusCounts {
            active,
            outdated,
            archived,
            merged,
            total: active + outdated + archived + merged,
        },
        policy_version: MEMORY_POLICY_VERSION,
    })
}

pub fn detail(database: &Database, memory_id: &str) -> Result<MemoryDetail, String> {
    Ok(MemoryDetail {
        memory: database
            .get_memory(memory_id)
            .map_err(|error| error.to_string())?,
        relations: database
            .list_memory_relations_for(memory_id)
            .map_err(|error| error.to_string())?,
        operations: database
            .list_memory_operations_for(memory_id, 100)
            .map_err(|error| error.to_string())?,
    })
}

pub fn user_edit(
    database: &Database,
    memory_id: &str,
    edit: UserMemoryEdit,
    now: i64,
) -> Result<Memory, String> {
    let previous = database
        .get_memory(memory_id)
        .map_err(|error| error.to_string())?;
    let content = edit.content.trim().to_string();
    if content.is_empty() || content.chars().count() > 2_000 {
        return Err("Memory content must contain 1 to 2000 characters".to_string());
    }
    let kind = edit.kind.unwrap_or_else(|| previous.kind.clone());
    validate_kind(&kind)?;
    let importance = edit.importance.unwrap_or(previous.importance);
    let confidence = edit.confidence.unwrap_or(previous.confidence);
    if !importance.is_finite() || !(0.0..=1.0).contains(&importance) {
        return Err("Memory importance must be between 0 and 1".to_string());
    }
    if !confidence.is_finite() || !(0.0..=1.0).contains(&confidence) {
        return Err("Memory confidence must be between 0 and 1".to_string());
    }

    let memory = database
        .update_memory_as_user(memory_id, &content, &kind, importance, confidence, now)
        .map_err(|error| error.to_string())?;
    database
        .insert_memory_operation(&operation(
            Some(memory_id.to_string()),
            "USER_EDIT",
            now,
            None,
            "user_correction",
            &[memory_id.to_string()],
            json!({
                "before": {
                    "content": previous.content,
                    "kind": previous.kind,
                    "importance": previous.importance,
                    "confidence": previous.confidence,
                    "sourceType": previous.source_type,
                    "sourceRef": previous.source_ref,
                },
                "after": {
                    "content": memory.content,
                    "kind": memory.kind,
                    "importance": memory.importance,
                    "confidence": memory.confidence,
                    "sourceType": memory.source_type,
                    "sourceRef": memory.source_ref,
                },
                "policyVersion": MEMORY_POLICY_VERSION,
            }),
        ))
        .map_err(|error| error.to_string())?;
    Ok(memory)
}

pub fn restore(database: &Database, memory_id: &str, now: i64) -> Result<Memory, String> {
    let memory = database
        .get_memory(memory_id)
        .map_err(|error| error.to_string())?;
    if memory.status == "active" {
        return Ok(memory);
    }
    if memory.status != "archived" {
        return Err("Only archived memories can be restored".to_string());
    }
    database
        .set_memory_status(memory_id, "active", now)
        .map_err(|error| error.to_string())?;
    database
        .insert_memory_operation(&operation(
            Some(memory_id.to_string()),
            "RESTORE",
            now,
            None,
            "user_restore",
            &[memory_id.to_string()],
            json!({"status": "active", "policyVersion": MEMORY_POLICY_VERSION}),
        ))
        .map_err(|error| error.to_string())?;
    database
        .get_memory(memory_id)
        .map_err(|error| error.to_string())
}

pub fn pin(database: &Database, memory_id: &str, now: i64) -> Result<Memory, String> {
    let previous = database
        .get_memory(memory_id)
        .map_err(|error| error.to_string())?;
    if previous.pinned {
        return Ok(previous);
    }
    let memory = database
        .pin_memory(memory_id, now)
        .map_err(|error| error.to_string())?;
    database
        .insert_memory_operation(&operation(
            Some(memory_id.to_string()),
            "PIN",
            now,
            None,
            "user_pin",
            &[memory_id.to_string()],
            json!({
                "importanceBefore": previous.importance,
                "importanceAfter": memory.importance,
                "policyVersion": MEMORY_POLICY_VERSION,
            }),
        ))
        .map_err(|error| error.to_string())?;
    Ok(memory)
}

pub fn delete(
    database: &Database,
    memory_id: &str,
    now: i64,
) -> Result<MemoryDeleteResult, String> {
    let memory = database
        .get_memory(memory_id)
        .map_err(|error| error.to_string())?;
    // Do not retain deleted user content in the audit row. The opaque id and
    // structural counts are enough to prove the user action occurred.
    let delete_operation = operation(
        None,
        "USER_DELETE",
        now,
        None,
        "user_delete",
        &[],
        json!({
            "deletedMemoryId": memory_id,
            "kind": memory.kind,
            "status": memory.status,
            "policyVersion": MEMORY_POLICY_VERSION,
        }),
    );
    let (relations_deleted, operations_deleted) = database
        .delete_memory_completely(memory_id, &delete_operation)
        .map_err(|error| error.to_string())?;
    Ok(MemoryDeleteResult {
        deleted_id: memory_id.to_string(),
        relations_deleted,
        operations_deleted,
    })
}

#[tauri::command]
pub fn write_memory(
    state: State<'_, AppState>,
    candidate: MemoryCandidate,
) -> Result<MemoryWriteResult, String> {
    write_candidate(&state.database, candidate)
}

#[tauri::command]
pub fn retrieve_memories(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<Memory>, String> {
    retrieve(&state.database, &query, limit.unwrap_or(6), timestamp_ms())
}

#[tauri::command]
pub fn get_memory_viewer(
    state: State<'_, AppState>,
    filter: Option<MemoryFilter>,
    operation_limit: Option<usize>,
) -> Result<MemoryViewerSnapshot, String> {
    viewer_snapshot(
        &state.database,
        filter.unwrap_or_default(),
        operation_limit.unwrap_or(100),
    )
}

#[tauri::command]
pub fn get_memory_detail(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<MemoryDetail, String> {
    detail(&state.database, &memory_id)
}

#[tauri::command]
pub fn consolidate_memories(state: State<'_, AppState>) -> Result<usize, String> {
    consolidate(&state.database, timestamp_ms())
}

#[tauri::command]
pub fn archive_memory(state: State<'_, AppState>, memory_id: String) -> Result<Memory, String> {
    archive(&state.database, &memory_id, timestamp_ms())
}

#[tauri::command]
pub fn edit_memory(
    state: State<'_, AppState>,
    memory_id: String,
    edit: UserMemoryEdit,
) -> Result<Memory, String> {
    user_edit(&state.database, &memory_id, edit, timestamp_ms())
}

#[tauri::command]
pub fn restore_memory(state: State<'_, AppState>, memory_id: String) -> Result<Memory, String> {
    restore(&state.database, &memory_id, timestamp_ms())
}

#[tauri::command]
pub fn delete_memory(
    state: State<'_, AppState>,
    memory_id: String,
) -> Result<MemoryDeleteResult, String> {
    delete(&state.database, &memory_id, timestamp_ms())
}

#[tauri::command]
pub fn pin_memory(state: State<'_, AppState>, memory_id: String) -> Result<Memory, String> {
    pin(&state.database, &memory_id, timestamp_ms())
}

#[tauri::command]
pub fn get_relationship_summary(state: State<'_, AppState>) -> Result<String, String> {
    let summary = state
        .database
        .list_active_memories()
        .map_err(|error| error.to_string())?
        .into_iter()
        .filter(|memory| memory.kind == "relationship")
        .take(3)
        .map(|memory| memory.content)
        .collect::<Vec<_>>()
        .join("; ");
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(content: &str, source_ref: &str) -> MemoryCandidate {
        MemoryCandidate {
            content: content.to_string(),
            source_ref: Some(source_ref.to_string()),
            source_type: Some("user_explicit".to_string()),
            occurred_at: Some(1_000_000),
        }
    }

    #[test]
    fn selectively_filters_and_classifies_candidates() {
        assert!(!worth_remembering("hello"));
        assert!(worth_remembering("User's cat is named Cream"));
        assert_eq!(classify("User prefers tea"), "semantic");
        assert_eq!(classify("Today the user started a new job"), "episodic");
        assert_eq!(
            classify("The user named the desktop pet Remi"),
            "relationship"
        );
    }

    #[test]
    fn preference_update_supersedes_stale_memory() {
        let database = Database::in_memory().unwrap();
        let coffee = write_candidate(&database, candidate("User drinks coffee", "message-1"))
            .unwrap()
            .memory
            .unwrap();
        let result = write_candidate(
            &database,
            candidate(
                "User doesn't drink coffee anymore and now prefers tea",
                "message-2",
            ),
        )
        .unwrap();
        assert_eq!(result.decision, "SUPERSEDE");
        assert_eq!(database.get_memory(&coffee.id).unwrap().status, "outdated");
        let active = retrieve(
            &database,
            "What does the user prefer to drink?",
            3,
            1_000_001,
        )
        .unwrap();
        assert!(active[0].content.contains("tea"));
        assert!(
            database
                .list_memory_relations()
                .unwrap()
                .iter()
                .any(|relation| {
                    relation.relation == "supersedes" && relation.target_id == coffee.id
                })
        );
    }

    #[test]
    fn temporary_event_has_validity_and_expires() {
        let database = Database::in_memory().unwrap();
        let memory = write_candidate(
            &database,
            candidate("User has an interview tomorrow", "message-time"),
        )
        .unwrap()
        .memory
        .unwrap();
        assert_eq!(memory.kind, "episodic");
        assert!(memory.valid_to.is_some());
        let after = memory.valid_to.unwrap() + 1;
        assert!(
            retrieve(&database, "interview", 3, after)
                .unwrap()
                .is_empty()
        );
        assert_eq!(database.get_memory(&memory.id).unwrap().status, "outdated");
    }

    #[test]
    fn provenance_distinguishes_explicit_and_inferred() {
        let database = Database::in_memory().unwrap();
        let explicit = write_candidate(&database, candidate("User lives in Shanghai", "m1"))
            .unwrap()
            .memory
            .unwrap();
        let mut inferred = candidate("User may enjoy quiet mornings", "reflection-1");
        inferred.source_type = Some("agent_inferred".to_string());
        let inferred = write_candidate(&database, inferred)
            .unwrap()
            .memory
            .unwrap();
        assert_eq!(explicit.source_type.as_deref(), Some("user_explicit"));
        assert_eq!(inferred.source_type.as_deref(), Some("agent_inferred"));
    }

    #[test]
    fn archived_memory_is_forgotten_by_retrieval() {
        let database = Database::in_memory().unwrap();
        let memory = write_candidate(
            &database,
            candidate("User's cat is named Nainiu", "forget-1"),
        )
        .unwrap()
        .memory
        .unwrap();
        archive(&database, &memory.id, 1_000_001).unwrap();
        assert!(
            retrieve(&database, "What is the cat named?", 3, 1_000_002)
                .unwrap()
                .is_empty()
        );
        assert_eq!(database.get_memory(&memory.id).unwrap().status, "archived");
    }

    #[test]
    fn inspector_searches_filters_and_returns_detail() {
        let database = Database::in_memory().unwrap();
        let shanghai = write_candidate(&database, candidate("User lives in Shanghai", "search-1"))
            .unwrap()
            .memory
            .unwrap();
        let cat = write_candidate(
            &database,
            candidate("User's cat is named Nainiu", "search-2"),
        )
        .unwrap()
        .memory
        .unwrap();
        archive(&database, &cat.id, 1_000_001).unwrap();

        let snapshot = viewer_snapshot(
            &database,
            MemoryFilter {
                kind: Some("semantic".to_string()),
                status: Some("active".to_string()),
                query: Some("SHANGHAI".to_string()),
            },
            20,
        )
        .unwrap();
        assert_eq!(snapshot.memories.len(), 1);
        assert_eq!(snapshot.memories[0].id, shanghai.id);
        assert_eq!(snapshot.counts.active, 1);
        assert_eq!(snapshot.counts.archived, 1);
        assert_eq!(snapshot.counts.total, 2);

        let detail = detail(&database, &shanghai.id).unwrap();
        assert_eq!(detail.memory.source_ref.as_deref(), Some("search-1"));
        assert!(detail.operations.iter().any(|item| item.operation == "ADD"));
    }

    #[test]
    fn user_edit_records_correction_provenance() {
        let database = Database::in_memory().unwrap();
        let memory = write_candidate(&database, candidate("User lives in Beijing", "edit-source"))
            .unwrap()
            .memory
            .unwrap();
        let edited = user_edit(
            &database,
            &memory.id,
            UserMemoryEdit {
                content: "User lives in Shanghai".to_string(),
                kind: None,
                importance: Some(0.85),
                confidence: Some(1.0),
            },
            1_000_002,
        )
        .unwrap();
        assert_eq!(edited.source_type.as_deref(), Some("user_correction"));
        assert_eq!(edited.source_ref.as_deref(), Some("memory-inspector"));
        assert_eq!(edited.content, "User lives in Shanghai");
        let operations = database.list_memory_operations_for(&memory.id, 20).unwrap();
        assert_eq!(operations[0].operation, "USER_EDIT");
        assert!(
            operations[0]
                .detail_json
                .as_deref()
                .is_some_and(|detail| detail.contains("User lives in Beijing"))
        );
    }

    #[test]
    fn archive_restore_and_pin_use_explicit_user_traces() {
        let database = Database::in_memory().unwrap();
        let pinned = write_candidate(
            &database,
            candidate("User's favorite color is blue", "pin-1"),
        )
        .unwrap()
        .memory
        .unwrap();
        let cat = write_candidate(&database, candidate("User's cat is named Nainiu", "pin-2"))
            .unwrap()
            .memory
            .unwrap();

        archive(&database, &cat.id, 1_000_001).unwrap();
        assert_eq!(database.get_memory(&cat.id).unwrap().status, "archived");
        restore(&database, &cat.id, 1_000_002).unwrap();
        assert_eq!(database.get_memory(&cat.id).unwrap().status, "active");

        let previous_importance = pinned.importance;
        let pinned = pin(&database, &pinned.id, 1_000_003).unwrap();
        assert!(pinned.pinned);
        assert!(pinned.importance > previous_importance);
        // Pinning raises only the normal importance factor. It does not force an
        // irrelevant memory into the result ahead of a relevant one.
        let selected = retrieve(&database, "What is the cat named?", 1, 1_000_004).unwrap();
        assert_eq!(selected[0].id, cat.id);

        let operation_names: HashSet<String> = database
            .list_memory_operations(20)
            .unwrap()
            .into_iter()
            .map(|item| item.operation)
            .collect();
        assert!(operation_names.contains("ARCHIVE"));
        assert!(operation_names.contains("RESTORE"));
        assert!(operation_names.contains("PIN"));
    }

    #[test]
    fn user_delete_removes_memory_relations_and_old_operations() {
        let database = Database::in_memory().unwrap();
        let coffee = write_candidate(&database, candidate("User drinks coffee", "delete-1"))
            .unwrap()
            .memory
            .unwrap();
        write_candidate(
            &database,
            candidate(
                "User doesn't drink coffee anymore and now prefers tea",
                "delete-2",
            ),
        )
        .unwrap();
        let result = delete(&database, &coffee.id, 1_000_003).unwrap();
        assert_eq!(result.deleted_id, coffee.id);
        assert!(result.relations_deleted >= 1);
        assert!(result.operations_deleted >= 1);
        assert!(database.get_memory(&coffee.id).is_err());
        assert!(
            database
                .list_memory_relations_for(&coffee.id)
                .unwrap()
                .is_empty()
        );

        let operations = database.list_memory_operations(50).unwrap();
        assert!(operations.iter().any(|item| {
            item.operation == "USER_DELETE"
                && item.memory_id.is_none()
                && item
                    .detail_json
                    .as_deref()
                    .is_some_and(|detail| detail.contains(&coffee.id))
        }));
        assert!(!operations.iter().any(|item| {
            item.memory_id.as_deref() == Some(&coffee.id)
                || item
                    .related_memory_ids_json
                    .as_deref()
                    .is_some_and(|ids| ids.contains(&coffee.id))
        }));
    }

    #[test]
    fn retrieves_persisted_memory_after_database_reopen() {
        let path = std::env::temp_dir().join(format!("remi-memory-{}.sqlite3", Uuid::new_v4()));
        {
            let database = Database::open(&path).unwrap();
            let result = write_candidate(
                &database,
                candidate("User's cat is named Nainiu", "message-1"),
            )
            .unwrap();
            assert_eq!(result.decision, "ADD");
        }
        {
            let database = Database::open(&path).unwrap();
            let memories = retrieve(
                &database,
                "What is the user's cat named?",
                3,
                timestamp_ms(),
            )
            .unwrap();
            assert_eq!(memories[0].content, "User's cat is named Nainiu");
        }
        let _ = std::fs::remove_file(path);
    }
}
