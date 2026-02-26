use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use argus_core::agent::RawDocument;
use argus_core::config::AppConfig;
use argus_core::entity::{Entity, EntityType, ExtractionResult, RelationType, Relationship};
use argus_core::error::{ArgusError, Result};
use argus_core::extraction::ExtractionPipeline;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-haiku-4-5-20251001";
const MAX_TOKENS: u32 = 4096;

/// LLM-based entity and relationship extraction pipeline using the Anthropic Messages API.
pub struct LlmExtractionPipeline {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

// ── Anthropic Messages API request/response types ──────────────────────────

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

// ── Intermediate JSON schema for LLM output parsing ────────────────────────

#[derive(Debug, Deserialize)]
struct LlmExtractionOutput {
    #[serde(default)]
    entities: Vec<LlmEntity>,
    #[serde(default)]
    relationships: Vec<LlmRelationship>,
}

#[derive(Debug, Deserialize)]
struct LlmEntity {
    name: String,
    #[serde(rename = "type")]
    entity_type: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    properties: serde_json::Value,
    #[serde(default = "default_confidence")]
    confidence: f64,
}

#[derive(Debug, Deserialize)]
struct LlmRelationship {
    source: String,
    target: String,
    #[serde(rename = "type")]
    relation_type: String,
    #[serde(default)]
    properties: serde_json::Value,
    #[serde(default = "default_confidence")]
    confidence: f64,
}

fn default_confidence() -> f64 {
    1.0
}

// ── Implementation ─────────────────────────────────────────────────────────

impl LlmExtractionPipeline {
    pub fn new(config: &AppConfig) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            api_key: config.anthropic_api_key.clone(),
            model: MODEL.to_string(),
        }
    }

    fn build_system_prompt() -> String {
        r#"You are an entity and relationship extraction system for an intelligence analysis platform.

Given a document, extract all notable entities and the relationships between them.

Return ONLY valid JSON (no markdown fences, no commentary) matching this exact schema:

{
  "entities": [
    {
      "name": "Entity Name",
      "type": "person | organization | vessel | aircraft | location | event | document | transaction | sanction",
      "aliases": ["optional alternate names"],
      "properties": { "arbitrary": "key-value pairs with extra info" },
      "confidence": 0.0 to 1.0
    }
  ],
  "relationships": [
    {
      "source": "Source Entity Name",
      "target": "Target Entity Name",
      "type": "owner_of | director_of | employee_of | related_to | located_at | transacted_with | sanctioned_by | registered_in | flagged_as | meeting_with | traveled_to | part_of",
      "properties": { "arbitrary": "key-value pairs" },
      "confidence": 0.0 to 1.0
    }
  ]
}

Rules:
- Entity names in relationships MUST exactly match an entity in the entities list.
- Choose the most specific entity type and relationship type that applies.
- Only extract entities and relationships that are clearly supported by the text.
- If no entities or relationships can be extracted, return {"entities": [], "relationships": []}.
- Output ONLY the JSON object. No additional text."#
            .to_string()
    }

    fn build_user_prompt(document: &RawDocument) -> String {
        let mut prompt = String::new();
        if let Some(title) = &document.title {
            prompt.push_str(&format!("Title: {}\n", title));
        }
        if let Some(url) = &document.url {
            prompt.push_str(&format!("URL: {}\n", url));
        }
        prompt.push_str(&format!("Source: {}\n", document.source));
        prompt.push_str(&format!("\nDocument content:\n{}", document.content));
        prompt
    }

    async fn call_anthropic(&self, document: &RawDocument) -> Result<String> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: MAX_TOKENS,
            system: Self::build_system_prompt(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Self::build_user_prompt(document),
            }],
        };

        tracing::debug!(
            model = %self.model,
            source = %document.source,
            content_len = document.content.len(),
            "Sending extraction request to Anthropic API"
        );

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ArgusError::Extraction(format!("HTTP request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            return Err(ArgusError::Extraction(format!(
                "Anthropic API returned status {status}: {body}"
            )));
        }

        let api_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ArgusError::Extraction(format!("Failed to parse API response: {e}")))?;

        // Extract the text from the first text content block
        let text = api_response
            .content
            .iter()
            .find_map(|block| {
                if block.block_type == "text" {
                    block.text.clone()
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                ArgusError::Extraction("No text content block in API response".to_string())
            })?;

        tracing::debug!(
            stop_reason = ?api_response.stop_reason,
            response_len = text.len(),
            "Received extraction response from Anthropic API"
        );

        Ok(text)
    }

    fn parse_entity_type(s: &str) -> EntityType {
        match s.to_lowercase().as_str() {
            "person" => EntityType::Person,
            "organization" | "org" | "company" => EntityType::Organization,
            "vessel" | "ship" | "boat" => EntityType::Vessel,
            "aircraft" | "plane" | "helicopter" => EntityType::Aircraft,
            "location" | "place" | "country" | "city" => EntityType::Location,
            "event" | "incident" => EntityType::Event,
            "document" | "report" | "filing" => EntityType::Document,
            "transaction" | "payment" | "transfer" => EntityType::Transaction,
            "sanction" | "sanctions" => EntityType::Sanction,
            _ => {
                tracing::warn!(entity_type = %s, "Unknown entity type, defaulting to Event");
                EntityType::Event
            }
        }
    }

    fn parse_relation_type(s: &str) -> RelationType {
        match s.to_lowercase().as_str() {
            "owner_of" | "owns" => RelationType::OwnerOf,
            "director_of" | "directs" => RelationType::DirectorOf,
            "employee_of" | "works_for" | "employed_by" => RelationType::EmployeeOf,
            "related_to" | "associated_with" => RelationType::RelatedTo,
            "located_at" | "located_in" | "based_in" => RelationType::LocatedAt,
            "transacted_with" | "traded_with" | "paid" => RelationType::TransactedWith,
            "sanctioned_by" | "sanctioned" => RelationType::SanctionedBy,
            "registered_in" | "incorporated_in" => RelationType::RegisteredIn,
            "flagged_as" | "flagged" => RelationType::FlaggedAs,
            "meeting_with" | "met_with" => RelationType::MeetingWith,
            "traveled_to" | "visited" => RelationType::TraveledTo,
            "part_of" | "member_of" | "subsidiary_of" => RelationType::PartOf,
            _ => {
                tracing::warn!(relation_type = %s, "Unknown relation type, defaulting to RelatedTo");
                RelationType::RelatedTo
            }
        }
    }

    fn parse_llm_response(
        raw_json: &str,
        source: &str,
    ) -> Result<(Vec<Entity>, Vec<Relationship>)> {
        // Strip potential markdown code fences the LLM might include despite instructions
        let cleaned = raw_json.trim();
        let cleaned = if cleaned.starts_with("```") {
            let start = cleaned.find('{').unwrap_or(0);
            let end = cleaned.rfind('}').map(|i| i + 1).unwrap_or(cleaned.len());
            &cleaned[start..end]
        } else {
            cleaned
        };

        let output: LlmExtractionOutput = serde_json::from_str(cleaned).map_err(|e| {
            tracing::error!(raw = %cleaned, error = %e, "Failed to parse LLM extraction JSON");
            ArgusError::Extraction(format!("Failed to parse LLM JSON output: {e}"))
        })?;

        let now = Utc::now();

        // Build entities and a name -> UUID lookup for relationship wiring
        let mut entities = Vec::with_capacity(output.entities.len());
        let mut name_to_id: HashMap<String, Uuid> = HashMap::new();

        for llm_entity in &output.entities {
            let id = Uuid::new_v4();
            let entity_type = Self::parse_entity_type(&llm_entity.entity_type);

            let entity = Entity {
                id,
                entity_type,
                name: llm_entity.name.clone(),
                aliases: llm_entity.aliases.clone(),
                properties: if llm_entity.properties.is_null() {
                    serde_json::Value::Object(serde_json::Map::new())
                } else {
                    llm_entity.properties.clone()
                },
                source: source.to_string(),
                source_id: None,
                confidence: llm_entity.confidence,
                first_seen: now,
                last_seen: now,
            };

            // Store canonical name (lowercased) for lookup
            name_to_id.insert(llm_entity.name.to_lowercase(), id);
            // Also store aliases
            for alias in &llm_entity.aliases {
                name_to_id.insert(alias.to_lowercase(), id);
            }

            entities.push(entity);
        }

        // Build relationships, resolving entity names to UUIDs
        let mut relationships = Vec::with_capacity(output.relationships.len());

        for llm_rel in &output.relationships {
            let source_id = name_to_id.get(&llm_rel.source.to_lowercase());
            let target_id = name_to_id.get(&llm_rel.target.to_lowercase());

            match (source_id, target_id) {
                (Some(&src), Some(&tgt)) => {
                    let relationship = Relationship {
                        id: Uuid::new_v4(),
                        source_entity_id: src,
                        target_entity_id: tgt,
                        relation_type: Self::parse_relation_type(&llm_rel.relation_type),
                        properties: if llm_rel.properties.is_null() {
                            serde_json::Value::Object(serde_json::Map::new())
                        } else {
                            llm_rel.properties.clone()
                        },
                        confidence: llm_rel.confidence,
                        source: source.to_string(),
                        timestamp: Some(now),
                    };
                    relationships.push(relationship);
                }
                _ => {
                    tracing::warn!(
                        source_name = %llm_rel.source,
                        target_name = %llm_rel.target,
                        source_found = source_id.is_some(),
                        target_found = target_id.is_some(),
                        "Skipping relationship: referenced entity not found"
                    );
                }
            }
        }

        tracing::info!(
            entities = entities.len(),
            relationships = relationships.len(),
            "Parsed extraction results"
        );

        Ok((entities, relationships))
    }
}

#[async_trait]
impl ExtractionPipeline for LlmExtractionPipeline {
    async fn extract(&self, document: &RawDocument) -> Result<ExtractionResult> {
        tracing::info!(
            source = %document.source,
            source_id = %document.source_id,
            title = ?document.title,
            "Starting entity extraction for document"
        );

        let raw_json = self.call_anthropic(document).await?;
        let (entities, relationships) = Self::parse_llm_response(&raw_json, &document.source)?;

        tracing::info!(
            source = %document.source,
            entities = entities.len(),
            relationships = relationships.len(),
            "Extraction complete"
        );

        Ok(ExtractionResult {
            entities,
            relationships,
            raw_source: document.source_id.clone(),
            extracted_at: Utc::now(),
        })
    }

    async fn extract_batch(&self, documents: &[RawDocument]) -> Result<Vec<ExtractionResult>> {
        tracing::info!(count = documents.len(), "Starting batch extraction");

        let mut join_set = tokio::task::JoinSet::new();

        for (i, doc) in documents.iter().enumerate() {
            let client = self.client.clone();
            let api_key = self.api_key.clone();
            let model = self.model.clone();
            let doc = doc.clone();

            join_set.spawn(async move {
                let pipeline = LlmExtractionPipeline {
                    client,
                    api_key,
                    model,
                };
                (i, pipeline.extract(&doc).await)
            });
        }

        let mut extraction_results = Vec::with_capacity(documents.len());
        let mut errors = Vec::new();

        while let Some(join_result) = join_set.join_next().await {
            match join_result {
                Ok((_i, Ok(extraction))) => extraction_results.push(extraction),
                Ok((i, Err(e))) => {
                    tracing::error!(
                        document_index = i,
                        source = %documents[i].source,
                        error = %e,
                        "Extraction failed for document in batch"
                    );
                    errors.push(format!(
                        "Document {} ({}): {}",
                        i, documents[i].source_id, e
                    ));
                }
                Err(join_err) => {
                    tracing::error!(error = %join_err, "Task panicked during batch extraction");
                    errors.push(format!("Task join error: {join_err}"));
                }
            }
        }

        if extraction_results.is_empty() && !errors.is_empty() {
            return Err(ArgusError::Extraction(format!(
                "All documents failed extraction: {}",
                errors.join("; ")
            )));
        }

        if !errors.is_empty() {
            tracing::warn!(
                succeeded = extraction_results.len(),
                failed = errors.len(),
                "Batch extraction completed with partial failures"
            );
        } else {
            tracing::info!(
                count = extraction_results.len(),
                "Batch extraction completed successfully"
            );
        }

        Ok(extraction_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entity_types() {
        assert_eq!(
            std::mem::discriminant(&LlmExtractionPipeline::parse_entity_type("person")),
            std::mem::discriminant(&EntityType::Person)
        );
        assert_eq!(
            std::mem::discriminant(&LlmExtractionPipeline::parse_entity_type("Organization")),
            std::mem::discriminant(&EntityType::Organization)
        );
        assert_eq!(
            std::mem::discriminant(&LlmExtractionPipeline::parse_entity_type("VESSEL")),
            std::mem::discriminant(&EntityType::Vessel)
        );
        assert_eq!(
            std::mem::discriminant(&LlmExtractionPipeline::parse_entity_type("unknown_thing")),
            std::mem::discriminant(&EntityType::Event)
        );
    }

    #[test]
    fn test_parse_relation_types() {
        assert_eq!(
            std::mem::discriminant(&LlmExtractionPipeline::parse_relation_type("owner_of")),
            std::mem::discriminant(&RelationType::OwnerOf)
        );
        assert_eq!(
            std::mem::discriminant(&LlmExtractionPipeline::parse_relation_type("located_in")),
            std::mem::discriminant(&RelationType::LocatedAt)
        );
        assert_eq!(
            std::mem::discriminant(&LlmExtractionPipeline::parse_relation_type("something_else")),
            std::mem::discriminant(&RelationType::RelatedTo)
        );
    }

    #[test]
    fn test_parse_llm_response_valid() {
        let json = r#"{
            "entities": [
                {
                    "name": "Acme Corp",
                    "type": "organization",
                    "aliases": ["ACME"],
                    "properties": {"industry": "defense"},
                    "confidence": 0.95
                },
                {
                    "name": "John Smith",
                    "type": "person",
                    "aliases": [],
                    "properties": {},
                    "confidence": 0.9
                }
            ],
            "relationships": [
                {
                    "source": "John Smith",
                    "target": "Acme Corp",
                    "type": "director_of",
                    "properties": {"since": "2020"},
                    "confidence": 0.85
                }
            ]
        }"#;

        let (entities, relationships) =
            LlmExtractionPipeline::parse_llm_response(json, "test").unwrap();

        assert_eq!(entities.len(), 2);
        assert_eq!(relationships.len(), 1);

        assert_eq!(entities[0].name, "Acme Corp");
        assert_eq!(entities[0].entity_type, EntityType::Organization);
        assert_eq!(entities[0].confidence, 0.95);
        assert_eq!(entities[0].aliases, vec!["ACME"]);

        assert_eq!(entities[1].name, "John Smith");
        assert_eq!(entities[1].entity_type, EntityType::Person);

        assert_eq!(relationships[0].source_entity_id, entities[1].id);
        assert_eq!(relationships[0].target_entity_id, entities[0].id);
        assert_eq!(relationships[0].relation_type, RelationType::DirectorOf);
        assert_eq!(relationships[0].confidence, 0.85);
    }

    #[test]
    fn test_parse_llm_response_with_code_fences() {
        let json = r#"```json
{
    "entities": [
        {"name": "TestEntity", "type": "location", "properties": {}, "confidence": 1.0}
    ],
    "relationships": []
}
```"#;

        let (entities, relationships) =
            LlmExtractionPipeline::parse_llm_response(json, "test").unwrap();

        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "TestEntity");
        assert_eq!(entities[0].entity_type, EntityType::Location);
        assert_eq!(relationships.len(), 0);
    }

    #[test]
    fn test_parse_llm_response_empty() {
        let json = r#"{"entities": [], "relationships": []}"#;
        let (entities, relationships) =
            LlmExtractionPipeline::parse_llm_response(json, "test").unwrap();

        assert_eq!(entities.len(), 0);
        assert_eq!(relationships.len(), 0);
    }

    #[test]
    fn test_parse_llm_response_missing_relationship_entity() {
        let json = r#"{
            "entities": [
                {"name": "Alpha", "type": "organization", "properties": {}, "confidence": 1.0}
            ],
            "relationships": [
                {
                    "source": "Alpha",
                    "target": "NonExistent",
                    "type": "related_to",
                    "properties": {},
                    "confidence": 0.5
                }
            ]
        }"#;

        let (entities, relationships) =
            LlmExtractionPipeline::parse_llm_response(json, "test").unwrap();

        assert_eq!(entities.len(), 1);
        // Relationship should be skipped because "NonExistent" is not in entities
        assert_eq!(relationships.len(), 0);
    }

    #[test]
    fn test_parse_llm_response_invalid_json() {
        let result = LlmExtractionPipeline::parse_llm_response("not json at all", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_llm_response_alias_lookup() {
        let json = r#"{
            "entities": [
                {
                    "name": "United States of America",
                    "type": "location",
                    "aliases": ["USA", "US"],
                    "properties": {},
                    "confidence": 1.0
                },
                {
                    "name": "Acme Corp",
                    "type": "organization",
                    "aliases": [],
                    "properties": {},
                    "confidence": 0.9
                }
            ],
            "relationships": [
                {
                    "source": "Acme Corp",
                    "target": "USA",
                    "type": "registered_in",
                    "properties": {},
                    "confidence": 0.8
                }
            ]
        }"#;

        let (entities, relationships) =
            LlmExtractionPipeline::parse_llm_response(json, "test").unwrap();

        assert_eq!(entities.len(), 2);
        // Relationship should resolve "USA" alias to the "United States of America" entity
        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].target_entity_id, entities[0].id);
    }
}
