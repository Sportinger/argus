use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use argus_core::entity::Entity;
use argus_core::error::{ArgusError, Result};
use argus_core::graph::{GraphQuery, GraphStore};
use argus_core::reasoning::{ReasoningEngine, ReasoningQuery, ReasoningResponse, ReasoningStep};
use argus_core::AppConfig;

// ---------------------------------------------------------------------------
// Anthropic Messages API types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    #[serde(default)]
    text: String,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-5-20250929";
const MAX_REASONING_ITERATIONS: usize = 5;

// ---------------------------------------------------------------------------
// Graph schema context used in prompts
// ---------------------------------------------------------------------------

const GRAPH_SCHEMA: &str = r#"
Node labels and properties:
  - Person { id, name, aliases, confidence, source, first_seen, last_seen, properties }
  - Organization { id, name, aliases, confidence, source, first_seen, last_seen, properties }
  - Vessel { id, name, aliases, confidence, source, first_seen, last_seen, properties }
  - Aircraft { id, name, aliases, confidence, source, first_seen, last_seen, properties }
  - Location { id, name, aliases, confidence, source, first_seen, last_seen, properties }
  - Event { id, name, aliases, confidence, source, first_seen, last_seen, properties }
  - Document { id, name, aliases, confidence, source, first_seen, last_seen, properties }
  - Transaction { id, name, aliases, confidence, source, first_seen, last_seen, properties }
  - Sanction { id, name, aliases, confidence, source, first_seen, last_seen, properties }

Relationship types:
  OWNER_OF, DIRECTOR_OF, EMPLOYEE_OF, RELATED_TO, LOCATED_AT,
  TRANSACTED_WITH, SANCTIONED_BY, REGISTERED_IN, FLAGGED_AS,
  MEETING_WITH, TRAVELED_TO, PART_OF

All relationships carry: { confidence, source, timestamp, properties }
"#;

// ---------------------------------------------------------------------------
// LlmReasoningEngine
// ---------------------------------------------------------------------------

pub struct LlmReasoningEngine {
    client: Client,
    graph: Arc<dyn GraphStore>,
    api_key: String,
}

impl LlmReasoningEngine {
    pub fn new(graph: Arc<dyn GraphStore>, config: &AppConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            graph,
            api_key: config.anthropic_api_key.clone(),
        }
    }

    // ------------------------------------------------------------------
    // Call the Anthropic Messages API
    // ------------------------------------------------------------------

    async fn call_llm(
        &self,
        system: &str,
        messages: &[Message],
        max_tokens: u32,
    ) -> Result<String> {
        let request = AnthropicRequest {
            model: MODEL.to_string(),
            max_tokens,
            messages: messages.to_vec(),
            system: Some(system.to_string()),
        };

        debug!(model = MODEL, "sending request to Anthropic API");

        let resp = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ArgusError::Reasoning(format!("HTTP request to Anthropic failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            return Err(ArgusError::Reasoning(format!(
                "Anthropic API returned {status}: {body}"
            )));
        }

        let api_resp: AnthropicResponse = resp
            .json()
            .await
            .map_err(|e| ArgusError::Reasoning(format!("failed to parse Anthropic response: {e}")))?;

        let text = api_resp
            .content
            .into_iter()
            .filter(|b| b.block_type == "text")
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        if text.is_empty() {
            return Err(ArgusError::Reasoning(
                "Anthropic returned an empty response".into(),
            ));
        }

        debug!(
            stop_reason = ?api_resp.stop_reason,
            response_len = text.len(),
            "received Anthropic API response"
        );

        Ok(text)
    }

    // ------------------------------------------------------------------
    // Step 1: Ask the LLM to generate Cypher queries for a question
    // ------------------------------------------------------------------

    fn build_cypher_generation_prompt(&self, question: &str, context: Option<&str>) -> String {
        let mut prompt = format!(
            "You are an expert Neo4j Cypher query writer for the ARGUS intelligence knowledge graph.\n\
             \n\
             {GRAPH_SCHEMA}\n\
             \n\
             Given the following question, generate one or more Cypher queries to retrieve the \
             relevant data from the graph. Return ONLY valid Cypher enclosed in ```cypher ... ``` \
             code blocks. Each query should be in its own code block.\n\
             If the question cannot be answered from the graph, return a single code block with \
             a broad search query that might find relevant entities.\n\n\
             Question: {question}"
        );

        if let Some(ctx) = context {
            prompt.push_str(&format!("\n\nAdditional context: {ctx}"));
        }

        prompt
    }

    fn extract_cypher_queries(response: &str) -> Vec<String> {
        let mut queries = Vec::new();
        let mut in_block = false;
        let mut current = String::new();

        for line in response.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("```cypher") || trimmed.starts_with("```CYPHER") {
                in_block = true;
                current.clear();
                continue;
            }
            if trimmed == "```" && in_block {
                in_block = false;
                let q = current.trim().to_string();
                if !q.is_empty() {
                    queries.push(q);
                }
                current.clear();
                continue;
            }
            if in_block {
                current.push_str(line);
                current.push('\n');
            }
        }

        // Fallback: if no fenced blocks found, try to find raw MATCH statements
        if queries.is_empty() {
            let mut raw = String::new();
            for line in response.lines() {
                let trimmed = line.trim();
                if trimmed.to_uppercase().starts_with("MATCH")
                    || trimmed.to_uppercase().starts_with("OPTIONAL")
                    || trimmed.to_uppercase().starts_with("WITH")
                    || trimmed.to_uppercase().starts_with("RETURN")
                    || trimmed.to_uppercase().starts_with("WHERE")
                    || trimmed.to_uppercase().starts_with("ORDER")
                    || trimmed.to_uppercase().starts_with("LIMIT")
                    || trimmed.to_uppercase().starts_with("CALL")
                {
                    raw.push_str(trimmed);
                    raw.push('\n');
                }
            }
            let q = raw.trim().to_string();
            if !q.is_empty() {
                queries.push(q);
            }
        }

        queries
    }

    // ------------------------------------------------------------------
    // Step 2: Execute Cypher queries against the graph store
    // ------------------------------------------------------------------

    async fn execute_queries(
        &self,
        queries: &[String],
    ) -> Vec<(String, std::result::Result<serde_json::Value, String>)> {
        let mut results = Vec::new();

        for cypher in queries {
            let graph_query = GraphQuery {
                cypher: cypher.clone(),
                params: serde_json::Value::Object(serde_json::Map::new()),
            };

            debug!(cypher = %cypher, "executing Cypher query on graph store");

            match self.graph.execute_cypher(&graph_query).await {
                Ok(val) => {
                    results.push((cypher.clone(), Ok(val)));
                }
                Err(e) => {
                    warn!(cypher = %cypher, error = %e, "Cypher query execution failed");
                    results.push((cypher.clone(), Err(e.to_string())));
                }
            }
        }

        results
    }

    // ------------------------------------------------------------------
    // Step 3: Feed results back to the LLM for interpretation
    // ------------------------------------------------------------------

    fn build_interpretation_prompt(
        question: &str,
        steps_summary: &str,
        context: Option<&str>,
    ) -> String {
        let mut prompt = format!(
            "You are an intelligence analyst using the ARGUS knowledge graph.\n\
             \n\
             A user asked the following question:\n\
             \"{question}\"\n\
             \n\
             The following Cypher queries were executed against the graph and their results are \
             shown below:\n\n\
             {steps_summary}\n\n\
             Based on these results, provide a comprehensive answer to the user's question.\n\n\
             Your response MUST follow this exact format:\n\
             \n\
             ANSWER: <your detailed answer>\n\
             CONFIDENCE: <a number between 0.0 and 1.0 reflecting how confident you are>\n\
             ENTITIES: <comma-separated list of entity names mentioned in the answer, or NONE>\n\
             SOURCES: <comma-separated list of data source identifiers referenced, or NONE>"
        );

        if let Some(ctx) = context {
            prompt.push_str(&format!("\n\nAdditional context: {ctx}"));
        }

        prompt
    }

    fn parse_interpretation(response: &str) -> (String, f64, Vec<String>, Vec<String>) {
        let mut answer = String::new();
        let mut confidence = 0.5_f64;
        let mut entities = Vec::new();
        let mut sources = Vec::new();

        // Parse structured fields from the response
        let mut current_section: Option<&str> = None;
        let mut answer_lines = Vec::new();

        for line in response.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("ANSWER:") {
                current_section = Some("answer");
                let v = rest.trim();
                if !v.is_empty() {
                    answer_lines.push(v.to_string());
                }
            } else if let Some(rest) = trimmed.strip_prefix("CONFIDENCE:") {
                current_section = Some("confidence");
                if let Ok(c) = rest.trim().parse::<f64>() {
                    confidence = c.clamp(0.0, 1.0);
                }
            } else if let Some(rest) = trimmed.strip_prefix("ENTITIES:") {
                current_section = Some("entities");
                let v = rest.trim();
                if !v.is_empty() && v.to_uppercase() != "NONE" {
                    entities = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }
            } else if let Some(rest) = trimmed.strip_prefix("SOURCES:") {
                current_section = Some("sources");
                let v = rest.trim();
                if !v.is_empty() && v.to_uppercase() != "NONE" {
                    sources = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }
            } else if current_section == Some("answer") {
                // Continuation lines of the answer
                answer_lines.push(trimmed.to_string());
            }
        }

        answer = answer_lines.join("\n").trim().to_string();

        // Fallback: if parsing failed, use the full response as the answer
        if answer.is_empty() {
            answer = response.trim().to_string();
        }

        (answer, confidence, entities, sources)
    }

    // ------------------------------------------------------------------
    // Resolve entity names to Entity objects via graph search
    // ------------------------------------------------------------------

    async fn resolve_entities(&self, names: &[String]) -> Vec<Entity> {
        let mut resolved = Vec::new();

        for name in names {
            match self.graph.search_entities(name, 1).await {
                Ok(mut found) => {
                    if let Some(entity) = found.pop() {
                        resolved.push(entity);
                    }
                }
                Err(e) => {
                    debug!(name = %name, error = %e, "could not resolve entity name from graph");
                }
            }
        }

        resolved
    }
}

// ---------------------------------------------------------------------------
// ReasoningEngine trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ReasoningEngine for LlmReasoningEngine {
    #[instrument(skip(self), fields(question = %query.question))]
    async fn query(&self, query: &ReasoningQuery) -> Result<ReasoningResponse> {
        info!(question = %query.question, "starting multi-step reasoning");

        let mut steps: Vec<ReasoningStep> = Vec::new();

        // ------------------------------------------------------------
        // Step 1: Generate Cypher queries from the user question
        // ------------------------------------------------------------
        let cypher_prompt = self.build_cypher_generation_prompt(
            &query.question,
            query.context.as_deref(),
        );

        let system = format!(
            "You are a Neo4j Cypher expert for the ARGUS intelligence knowledge graph.\n{GRAPH_SCHEMA}"
        );

        let messages = vec![Message {
            role: "user".to_string(),
            content: cypher_prompt,
        }];

        let cypher_response = self.call_llm(&system, &messages, 2048).await?;

        let cypher_queries = Self::extract_cypher_queries(&cypher_response);

        info!(
            num_queries = cypher_queries.len(),
            "LLM generated Cypher queries"
        );

        steps.push(ReasoningStep {
            description: "Generated Cypher queries from user question".to_string(),
            cypher: if cypher_queries.is_empty() {
                None
            } else {
                Some(cypher_queries.join(";\n"))
            },
            result_summary: format!("Generated {} Cypher queries", cypher_queries.len()),
        });

        if cypher_queries.is_empty() {
            return Err(ArgusError::Reasoning(
                "LLM did not produce any Cypher queries for the given question".into(),
            ));
        }

        // ------------------------------------------------------------
        // Step 2: Execute Cypher queries
        // ------------------------------------------------------------
        let query_results = self.execute_queries(&cypher_queries).await;

        let mut steps_summary = String::new();

        for (i, (cypher, result)) in query_results.iter().enumerate() {
            let (summary, result_str) = match result {
                Ok(val) => {
                    let json_str = serde_json::to_string_pretty(val)
                        .unwrap_or_else(|_| val.to_string());
                    // Truncate very large results to avoid exceeding token limits
                    let truncated = if json_str.len() > 4000 {
                        format!("{}... [truncated, {} total chars]", &json_str[..4000], json_str.len())
                    } else {
                        json_str.clone()
                    };
                    (
                        format!("Query {} returned results ({} chars)", i + 1, json_str.len()),
                        truncated,
                    )
                }
                Err(e) => (
                    format!("Query {} failed: {}", i + 1, e),
                    format!("Error: {e}"),
                ),
            };

            steps_summary.push_str(&format!(
                "--- Query {} ---\nCypher: {cypher}\nResult:\n{result_str}\n\n",
                i + 1
            ));

            steps.push(ReasoningStep {
                description: format!("Executed Cypher query {}", i + 1),
                cypher: Some(cypher.clone()),
                result_summary: summary,
            });
        }

        // ------------------------------------------------------------
        // Step 3: Iterative refinement — if all queries failed or
        //         returned empty results, let the LLM try again
        // ------------------------------------------------------------
        let all_empty_or_failed = query_results.iter().all(|(_, r)| match r {
            Ok(val) => val.is_null() || val == &serde_json::Value::Array(vec![]),
            Err(_) => true,
        });

        let mut final_steps_summary = steps_summary.clone();
        let mut iteration = 0;

        if all_empty_or_failed && iteration < MAX_REASONING_ITERATIONS {
            debug!("initial queries returned no data; attempting refinement");

            let refinement_system = format!(
                "You are a Neo4j Cypher expert for the ARGUS intelligence knowledge graph.\n{GRAPH_SCHEMA}"
            );
            let refinement_prompt = format!(
                "The following Cypher queries were executed but returned empty or errored results:\n\n\
                 {final_steps_summary}\n\n\
                 The original question was: \"{}\"\n\n\
                 Please generate alternative, broader Cypher queries that might find relevant data. \
                 Return ONLY valid Cypher enclosed in ```cypher ... ``` code blocks.",
                query.question
            );

            let refinement_messages = vec![Message {
                role: "user".to_string(),
                content: refinement_prompt,
            }];

            if let Ok(refinement_resp) = self.call_llm(&refinement_system, &refinement_messages, 2048).await {
                let refined_queries = Self::extract_cypher_queries(&refinement_resp);

                if !refined_queries.is_empty() {
                    info!(
                        num_queries = refined_queries.len(),
                        "LLM generated refined Cypher queries"
                    );

                    steps.push(ReasoningStep {
                        description: "Generated refined Cypher queries after initial results were empty".to_string(),
                        cypher: Some(refined_queries.join(";\n")),
                        result_summary: format!("Generated {} refined queries", refined_queries.len()),
                    });

                    let refined_results = self.execute_queries(&refined_queries).await;

                    for (i, (cypher, result)) in refined_results.iter().enumerate() {
                        let (summary, result_str) = match result {
                            Ok(val) => {
                                let json_str = serde_json::to_string_pretty(val)
                                    .unwrap_or_else(|_| val.to_string());
                                let truncated = if json_str.len() > 4000 {
                                    format!("{}... [truncated, {} total chars]", &json_str[..4000], json_str.len())
                                } else {
                                    json_str.clone()
                                };
                                (
                                    format!("Refined query {} returned results ({} chars)", i + 1, json_str.len()),
                                    truncated,
                                )
                            }
                            Err(e) => (
                                format!("Refined query {} failed: {}", i + 1, e),
                                format!("Error: {e}"),
                            ),
                        };

                        final_steps_summary.push_str(&format!(
                            "--- Refined Query {} ---\nCypher: {cypher}\nResult:\n{result_str}\n\n",
                            i + 1
                        ));

                        steps.push(ReasoningStep {
                            description: format!("Executed refined Cypher query {}", i + 1),
                            cypher: Some(cypher.clone()),
                            result_summary: summary,
                        });
                    }

                    iteration += 1;
                }
            }
        }
        // Suppress the unused assignment warning — `iteration` tracks refinement rounds and
        // would be read if the loop were extended.
        let _ = iteration;

        // ------------------------------------------------------------
        // Step 4: Interpret results with the LLM
        // ------------------------------------------------------------
        let interpretation_prompt = Self::build_interpretation_prompt(
            &query.question,
            &final_steps_summary,
            query.context.as_deref(),
        );

        let interp_system =
            "You are an intelligence analyst. Provide clear, evidence-based answers.".to_string();

        let interp_messages = vec![Message {
            role: "user".to_string(),
            content: interpretation_prompt,
        }];

        let interpretation = self.call_llm(&interp_system, &interp_messages, 4096).await?;

        let (answer, confidence, entity_names, sources) =
            Self::parse_interpretation(&interpretation);

        steps.push(ReasoningStep {
            description: "Interpreted graph results and formulated answer".to_string(),
            cypher: None,
            result_summary: format!(
                "Confidence: {confidence:.2}, entities referenced: {}",
                entity_names.len()
            ),
        });

        // ------------------------------------------------------------
        // Step 5: Resolve entity names to Entity objects
        // ------------------------------------------------------------
        let entities_referenced = self.resolve_entities(&entity_names).await;

        info!(
            answer_len = answer.len(),
            confidence = confidence,
            steps = steps.len(),
            entities = entities_referenced.len(),
            "reasoning complete"
        );

        Ok(ReasoningResponse {
            answer,
            confidence,
            steps,
            entities_referenced,
            sources,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cypher_queries_fenced() {
        let response = r#"
Here are the queries:

```cypher
MATCH (p:Person)-[:OWNER_OF]->(o:Organization) RETURN p, o LIMIT 10
```

```cypher
MATCH (e:Event) WHERE e.name CONTAINS 'summit' RETURN e
```
"#;
        let queries = LlmReasoningEngine::extract_cypher_queries(response);
        assert_eq!(queries.len(), 2);
        assert!(queries[0].contains("MATCH (p:Person)"));
        assert!(queries[1].contains("summit"));
    }

    #[test]
    fn test_extract_cypher_queries_fallback() {
        let response = "MATCH (n:Person) WHERE n.name = 'Alice' RETURN n LIMIT 5";
        let queries = LlmReasoningEngine::extract_cypher_queries(response);
        assert_eq!(queries.len(), 1);
        assert!(queries[0].contains("MATCH"));
    }

    #[test]
    fn test_extract_cypher_queries_empty() {
        let response = "I'm sorry, I cannot generate a query for that.";
        let queries = LlmReasoningEngine::extract_cypher_queries(response);
        assert!(queries.is_empty());
    }

    #[test]
    fn test_parse_interpretation_full() {
        let response = r#"ANSWER: The entity John Doe is connected to Acme Corp through a directorship.
CONFIDENCE: 0.85
ENTITIES: John Doe, Acme Corp
SOURCES: ofac_sdn, un_sanctions"#;

        let (answer, confidence, entities, sources) =
            LlmReasoningEngine::parse_interpretation(response);

        assert!(answer.contains("John Doe"));
        assert!((confidence - 0.85).abs() < f64::EPSILON);
        assert_eq!(entities.len(), 2);
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn test_parse_interpretation_fallback() {
        let response = "Some unstructured response without markers.";

        let (answer, confidence, entities, sources) =
            LlmReasoningEngine::parse_interpretation(response);

        assert_eq!(answer, "Some unstructured response without markers.");
        assert!((confidence - 0.5).abs() < f64::EPSILON);
        assert!(entities.is_empty());
        assert!(sources.is_empty());
    }

    #[test]
    fn test_parse_interpretation_clamps_confidence() {
        let response = "ANSWER: test\nCONFIDENCE: 1.5\nENTITIES: NONE\nSOURCES: NONE";

        let (_, confidence, _, _) = LlmReasoningEngine::parse_interpretation(response);
        assert!((confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_interpretation_multiline_answer() {
        let response = "ANSWER: Line one.\nLine two continues the answer.\nLine three as well.\nCONFIDENCE: 0.7\nENTITIES: NONE\nSOURCES: NONE";

        let (answer, confidence, _, _) = LlmReasoningEngine::parse_interpretation(response);
        assert!(answer.contains("Line one."));
        assert!(answer.contains("Line two"));
        assert!(answer.contains("Line three"));
        assert!((confidence - 0.7).abs() < f64::EPSILON);
    }
}
