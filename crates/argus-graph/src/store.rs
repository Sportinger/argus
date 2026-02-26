use async_trait::async_trait;
use neo4rs::{query, Graph, Node};
use std::future::Future;
use uuid::Uuid;

use argus_core::config::AppConfig;
use argus_core::entity::{Entity, EntityType, ExtractionResult, RelationType, Relationship};
use argus_core::error::{ArgusError, Result};
use argus_core::graph::{GraphNeighbors, GraphQuery, GraphStore};

/// Timeout for all Neo4j operations (seconds).
const NEO4J_TIMEOUT_SECS: u64 = 5;

pub struct Neo4jGraphStore {
    graph: Option<Graph>,
}

impl Neo4jGraphStore {
    pub async fn new(config: &AppConfig) -> Self {
        match Graph::new(&config.neo4j_uri, &config.neo4j_user, &config.neo4j_password).await {
            Ok(graph) => {
                tracing::info!(uri = %config.neo4j_uri, "Connected to Neo4j");
                Self { graph: Some(graph) }
            }
            Err(e) => {
                tracing::warn!(uri = %config.neo4j_uri, error = %e, "Failed to connect to Neo4j — running in degraded mode");
                Self { graph: None }
            }
        }
    }

    fn graph(&self) -> Result<&Graph> {
        self.graph.as_ref().ok_or_else(|| ArgusError::Graph("Neo4j not connected".into()))
    }

    pub fn is_connected(&self) -> bool {
        self.graph.is_some()
    }

}

/// Wrap any async operation with a timeout, converting timeout to ArgusError::Graph.
async fn timed<T, F: Future<Output = T>>(op: F) -> std::result::Result<T, ArgusError> {
    tokio::time::timeout(std::time::Duration::from_secs(NEO4J_TIMEOUT_SECS), op)
        .await
        .map_err(|_| {
            tracing::warn!("Neo4j operation timed out after {}s", NEO4J_TIMEOUT_SECS);
            ArgusError::Graph(format!("Neo4j operation timed out after {}s", NEO4J_TIMEOUT_SECS))
        })
}

fn entity_type_to_label(et: &EntityType) -> &'static str {
    match et {
        EntityType::Person => "Person",
        EntityType::Organization => "Organization",
        EntityType::Vessel => "Vessel",
        EntityType::Aircraft => "Aircraft",
        EntityType::Location => "Location",
        EntityType::Event => "Event",
        EntityType::Document => "Document",
        EntityType::Transaction => "Transaction",
        EntityType::Sanction => "Sanction",
    }
}

fn label_to_entity_type(label: &str) -> EntityType {
    match label {
        "Person" => EntityType::Person,
        "Organization" => EntityType::Organization,
        "Vessel" => EntityType::Vessel,
        "Aircraft" => EntityType::Aircraft,
        "Location" => EntityType::Location,
        "Event" => EntityType::Event,
        "Document" => EntityType::Document,
        "Transaction" => EntityType::Transaction,
        "Sanction" => EntityType::Sanction,
        _ => EntityType::Event,
    }
}

fn relation_type_to_label(rt: &RelationType) -> &'static str {
    match rt {
        RelationType::OwnerOf => "OWNER_OF",
        RelationType::DirectorOf => "DIRECTOR_OF",
        RelationType::EmployeeOf => "EMPLOYEE_OF",
        RelationType::RelatedTo => "RELATED_TO",
        RelationType::LocatedAt => "LOCATED_AT",
        RelationType::TransactedWith => "TRANSACTED_WITH",
        RelationType::SanctionedBy => "SANCTIONED_BY",
        RelationType::RegisteredIn => "REGISTERED_IN",
        RelationType::FlaggedAs => "FLAGGED_AS",
        RelationType::MeetingWith => "MEETING_WITH",
        RelationType::TraveledTo => "TRAVELED_TO",
        RelationType::PartOf => "PART_OF",
    }
}

fn label_to_relation_type(label: &str) -> RelationType {
    match label {
        "OWNER_OF" => RelationType::OwnerOf,
        "DIRECTOR_OF" => RelationType::DirectorOf,
        "EMPLOYEE_OF" => RelationType::EmployeeOf,
        "RELATED_TO" => RelationType::RelatedTo,
        "LOCATED_AT" => RelationType::LocatedAt,
        "TRANSACTED_WITH" => RelationType::TransactedWith,
        "SANCTIONED_BY" => RelationType::SanctionedBy,
        "REGISTERED_IN" => RelationType::RegisteredIn,
        "FLAGGED_AS" => RelationType::FlaggedAs,
        "MEETING_WITH" => RelationType::MeetingWith,
        "TRAVELED_TO" => RelationType::TraveledTo,
        "PART_OF" => RelationType::PartOf,
        _ => RelationType::RelatedTo,
    }
}

fn node_to_entity(node: &Node) -> Result<Entity> {
    let id_str: String = node
        .get("id")
        .map_err(|e| ArgusError::Graph(format!("Missing id on node: {}", e)))?;
    let id = Uuid::parse_str(&id_str)
        .map_err(|e| ArgusError::Graph(format!("Invalid UUID: {}", e)))?;

    let labels = node.labels();
    let entity_type = labels
        .first()
        .map(|l| label_to_entity_type(l))
        .unwrap_or(EntityType::Event);

    let name: String = node
        .get("name")
        .map_err(|e| ArgusError::Graph(format!("Missing name on node: {}", e)))?;

    let aliases_json: String = node.get("aliases").unwrap_or_else(|_| "[]".to_string());
    let aliases: Vec<String> =
        serde_json::from_str(&aliases_json).unwrap_or_default();

    let properties_json: String = node.get("properties").unwrap_or_else(|_| "{}".to_string());
    let properties: serde_json::Value =
        serde_json::from_str(&properties_json).unwrap_or(serde_json::Value::Object(Default::default()));

    let source: String = node.get("source").unwrap_or_else(|_| String::new());
    let source_id: Option<String> = node.get("source_id").ok();
    let confidence: f64 = node.get("confidence").unwrap_or(1.0);

    let first_seen_str: String = node
        .get("first_seen")
        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
    let first_seen = chrono::DateTime::parse_from_rfc3339(&first_seen_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let last_seen_str: String = node
        .get("last_seen")
        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339());
    let last_seen = chrono::DateTime::parse_from_rfc3339(&last_seen_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    Ok(Entity {
        id,
        entity_type,
        name,
        aliases,
        properties,
        source,
        source_id,
        confidence,
        first_seen,
        last_seen,
    })
}

#[async_trait]
impl GraphStore for Neo4jGraphStore {
    async fn store_extraction(&self, result: &ExtractionResult) -> Result<()> {
        let mut txn = timed(self.graph()?.start_txn())
            .await?
            .map_err(|e| ArgusError::Graph(format!("Failed to start transaction: {}", e)))?;

        for entity in &result.entities {
            let label = entity_type_to_label(&entity.entity_type);
            let aliases_json = serde_json::to_string(&entity.aliases)
                .map_err(|e| ArgusError::Graph(format!("Failed to serialize aliases: {}", e)))?;
            let properties_json = serde_json::to_string(&entity.properties)
                .map_err(|e| ArgusError::Graph(format!("Failed to serialize properties: {}", e)))?;

            // Cross-source entity resolution: first check if an entity with the
            // same name (case-insensitive) and type already exists from any source.
            // If found, merge onto that node and accumulate sources.
            // Otherwise, MERGE on (source, source_id) or (id) as before.
            let cypher = if entity.source_id.is_some() {
                format!(
                    "OPTIONAL MATCH (existing:{label} \
                       WHERE toLower(existing.name) = toLower($name) \
                       AND existing.source <> $source) \
                     WITH existing \
                     FOREACH (_ IN CASE WHEN existing IS NOT NULL THEN [1] ELSE [] END | \
                       SET existing.sources = CASE \
                         WHEN existing.sources IS NULL THEN [$source] \
                         WHEN NOT $source IN existing.sources THEN existing.sources + $source \
                         ELSE existing.sources END, \
                       existing.aliases = $aliases, \
                       existing.properties = $properties, \
                       existing.confidence = CASE WHEN $confidence > existing.confidence THEN $confidence ELSE existing.confidence END, \
                       existing.last_seen = $last_seen \
                     ) \
                     WITH existing \
                     FOREACH (_ IN CASE WHEN existing IS NULL THEN [1] ELSE [] END | \
                       MERGE (n:{label} {{source: $source, source_id: $source_id}}) \
                       ON CREATE SET n.id = $id, n.name = $name, n.aliases = $aliases, \
                         n.properties = $properties, n.confidence = $confidence, \
                         n.first_seen = $first_seen, n.last_seen = $last_seen, \
                         n.sources = [$source] \
                       ON MATCH SET n.name = $name, n.aliases = $aliases, \
                         n.properties = $properties, n.confidence = $confidence, \
                         n.last_seen = $last_seen, \
                         n.sources = CASE \
                           WHEN n.sources IS NULL THEN [$source] \
                           WHEN NOT $source IN n.sources THEN n.sources + $source \
                           ELSE n.sources END \
                     )",
                )
            } else {
                format!(
                    "OPTIONAL MATCH (existing:{label} \
                       WHERE toLower(existing.name) = toLower($name) \
                       AND existing.source <> $source) \
                     WITH existing \
                     FOREACH (_ IN CASE WHEN existing IS NOT NULL THEN [1] ELSE [] END | \
                       SET existing.sources = CASE \
                         WHEN existing.sources IS NULL THEN [$source] \
                         WHEN NOT $source IN existing.sources THEN existing.sources + $source \
                         ELSE existing.sources END, \
                       existing.aliases = $aliases, \
                       existing.properties = $properties, \
                       existing.confidence = CASE WHEN $confidence > existing.confidence THEN $confidence ELSE existing.confidence END, \
                       existing.last_seen = $last_seen \
                     ) \
                     WITH existing \
                     FOREACH (_ IN CASE WHEN existing IS NULL THEN [1] ELSE [] END | \
                       MERGE (n:{label} {{id: $id}}) \
                       ON CREATE SET n.name = $name, n.source = $source, n.source_id = $source_id, \
                         n.aliases = $aliases, n.properties = $properties, \
                         n.confidence = $confidence, n.first_seen = $first_seen, \
                         n.last_seen = $last_seen, \
                         n.sources = [$source] \
                       ON MATCH SET n.name = $name, n.aliases = $aliases, \
                         n.properties = $properties, n.confidence = $confidence, \
                         n.last_seen = $last_seen, \
                         n.sources = CASE \
                           WHEN n.sources IS NULL THEN [$source] \
                           WHEN NOT $source IN n.sources THEN n.sources + $source \
                           ELSE n.sources END \
                     )",
                )
            };

            let q = query(&cypher)
                .param("id", entity.id.to_string())
                .param("name", entity.name.clone())
                .param("source", entity.source.clone())
                .param(
                    "source_id",
                    entity.source_id.clone().unwrap_or_default(),
                )
                .param("aliases", aliases_json)
                .param("properties", properties_json)
                .param("confidence", entity.confidence)
                .param("first_seen", entity.first_seen.to_rfc3339())
                .param("last_seen", entity.last_seen.to_rfc3339());

            txn.run(q)
                .await
                .map_err(|e| ArgusError::Graph(format!("Failed to store entity {}: {}", entity.id, e)))?;

            tracing::debug!(
                entity_id = %entity.id,
                entity_name = %entity.name,
                entity_type = label,
                "Stored entity"
            );
        }

        for rel in &result.relationships {
            let rel_label = relation_type_to_label(&rel.relation_type);
            let properties_json = serde_json::to_string(&rel.properties)
                .map_err(|e| ArgusError::Graph(format!("Failed to serialize relationship properties: {}", e)))?;

            let timestamp_str = rel
                .timestamp
                .map(|t| t.to_rfc3339())
                .unwrap_or_default();

            // Use MERGE instead of CREATE to prevent duplicate relationships
            let cypher = format!(
                "MATCH (a {{id: $source_id}}) \
                 MATCH (b {{id: $target_id}}) \
                 MERGE (a)-[r:{} {{source: $source}}]->(b) \
                 ON CREATE SET r.id = $rel_id, r.properties = $properties, \
                   r.confidence = $confidence, r.timestamp = $timestamp \
                 ON MATCH SET r.properties = $properties, \
                   r.confidence = CASE WHEN $confidence > r.confidence THEN $confidence ELSE r.confidence END, \
                   r.timestamp = CASE WHEN $timestamp <> '' THEN $timestamp ELSE r.timestamp END",
                rel_label
            );

            let q = query(&cypher)
                .param("source_id", rel.source_entity_id.to_string())
                .param("target_id", rel.target_entity_id.to_string())
                .param("rel_id", rel.id.to_string())
                .param("properties", properties_json)
                .param("confidence", rel.confidence)
                .param("source", rel.source.clone())
                .param("timestamp", timestamp_str);

            txn.run(q)
                .await
                .map_err(|e| ArgusError::Graph(format!("Failed to store relationship {}: {}", rel.id, e)))?;

            tracing::debug!(
                rel_id = %rel.id,
                source = %rel.source_entity_id,
                target = %rel.target_entity_id,
                rel_type = rel_label,
                "Stored relationship"
            );
        }

        txn.commit()
            .await
            .map_err(|e| ArgusError::Graph(format!("Failed to commit transaction: {}", e)))?;

        tracing::info!(
            entities = result.entities.len(),
            relationships = result.relationships.len(),
            "Stored extraction result"
        );

        Ok(())
    }

    async fn get_entity(&self, id: Uuid) -> Result<Option<Entity>> {
        let mut stream = timed(
            self.graph()?
                .execute(query("MATCH (n {id: $id}) RETURN n").param("id", id.to_string())),
        )
        .await?
        .map_err(|e| ArgusError::Graph(format!("Failed to query entity: {}", e)))?;

        match stream.next().await {
            Ok(Some(row)) => {
                let node: Node = row
                    .get("n")
                    .map_err(|e| ArgusError::Graph(format!("Failed to deserialize node: {}", e)))?;
                let entity = node_to_entity(&node)?;
                Ok(Some(entity))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ArgusError::Graph(format!("Error fetching entity: {}", e))),
        }
    }

    async fn search_entities(&self, query_str: &str, limit: usize) -> Result<Vec<Entity>> {
        let cypher = "MATCH (n) WHERE n.name CONTAINS $query RETURN n LIMIT $limit";
        let q = query(cypher)
            .param("query", query_str.to_string())
            .param("limit", limit as i64);

        let mut stream = timed(self.graph()?.execute(q))
            .await?
            .map_err(|e| ArgusError::Graph(format!("Failed to search entities: {}", e)))?;

        let mut entities = Vec::new();
        while let Ok(Some(row)) = stream.next().await {
            let node: Node = row
                .get("n")
                .map_err(|e| ArgusError::Graph(format!("Failed to deserialize node: {}", e)))?;
            match node_to_entity(&node) {
                Ok(entity) => entities.push(entity),
                Err(e) => {
                    tracing::warn!(error = %e, "Skipping malformed entity node");
                }
            }
        }

        tracing::debug!(
            query = query_str,
            results = entities.len(),
            "Entity search completed"
        );

        Ok(entities)
    }

    async fn get_neighbors(&self, entity_id: Uuid, depth: u32) -> Result<GraphNeighbors> {
        // First get the root entity
        let root_entity = self
            .get_entity(entity_id)
            .await?
            .ok_or_else(|| ArgusError::NotFound(format!("Entity {} not found", entity_id)))?;

        let cypher = format!(
            "MATCH (n {{id: $id}})-[r*1..{}]-(m) \
             RETURN DISTINCT m, \
                    [rel IN r | type(rel)] AS rel_types, \
                    [rel IN r | properties(rel)] AS rel_props, \
                    [rel IN r | startNode(rel).id] AS rel_sources, \
                    [rel IN r | endNode(rel).id] AS rel_targets",
            depth
        );

        let q = query(&cypher).param("id", entity_id.to_string());

        let mut stream = timed(self.graph()?.execute(q))
            .await?
            .map_err(|e| ArgusError::Graph(format!("Failed to get neighbors: {}", e)))?;

        let mut neighbors = Vec::new();
        let mut relationships = Vec::new();
        let mut seen_neighbor_ids = std::collections::HashSet::new();
        let mut seen_rel_ids = std::collections::HashSet::new();

        while let Ok(Some(row)) = stream.next().await {
            // Parse neighbor node
            let neighbor_node: Node = match row.get("m") {
                Ok(n) => n,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to parse neighbor node");
                    continue;
                }
            };

            match node_to_entity(&neighbor_node) {
                Ok(neighbor) => {
                    if seen_neighbor_ids.insert(neighbor.id) {
                        neighbors.push(neighbor);
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Skipping malformed neighbor node");
                    continue;
                }
            }

            // Parse relationship chain types
            let rel_types: Vec<String> = row.get("rel_types").unwrap_or_default();
            let rel_sources: Vec<String> = row.get("rel_sources").unwrap_or_default();
            let rel_targets: Vec<String> = row.get("rel_targets").unwrap_or_default();
            let rel_props: Vec<serde_json::Value> = row.get("rel_props").unwrap_or_default();

            for i in 0..rel_types.len() {
                let rel_type = label_to_relation_type(&rel_types[i]);

                let source_id = rel_sources
                    .get(i)
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or(entity_id);
                let target_id = rel_targets
                    .get(i)
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .unwrap_or(entity_id);

                // Extract rel id from properties if available
                let props = rel_props.get(i).cloned().unwrap_or(serde_json::Value::Object(Default::default()));
                let rel_id_str = props
                    .as_object()
                    .and_then(|m| m.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let rel_id = Uuid::parse_str(rel_id_str).unwrap_or_else(|_| Uuid::new_v4());

                if !seen_rel_ids.insert(rel_id) {
                    continue;
                }

                let confidence = props
                    .as_object()
                    .and_then(|m| m.get("confidence"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0);

                let source = props
                    .as_object()
                    .and_then(|m| m.get("source"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let timestamp_str = props
                    .as_object()
                    .and_then(|m| m.get("timestamp"))
                    .and_then(|v| v.as_str());
                let timestamp = timestamp_str.and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .ok()
                });

                let inner_props = props
                    .as_object()
                    .and_then(|m| m.get("properties"))
                    .and_then(|v| serde_json::from_str(v.as_str().unwrap_or("{}")).ok())
                    .unwrap_or(serde_json::Value::Object(Default::default()));

                relationships.push(Relationship {
                    id: rel_id,
                    source_entity_id: source_id,
                    target_entity_id: target_id,
                    relation_type: rel_type,
                    properties: inner_props,
                    confidence,
                    source,
                    timestamp,
                });
            }
        }

        tracing::debug!(
            entity_id = %entity_id,
            depth = depth,
            neighbor_count = neighbors.len(),
            relationship_count = relationships.len(),
            "Fetched neighbors"
        );

        Ok(GraphNeighbors {
            entity: root_entity,
            relationships,
            neighbors,
        })
    }

    async fn execute_cypher(&self, graph_query: &GraphQuery) -> Result<serde_json::Value> {
        let mut q = query(&graph_query.cypher);

        // Add params from the JSON value
        if let Some(obj) = graph_query.params.as_object() {
            for (key, value) in obj {
                q = match value {
                    serde_json::Value::String(s) => q.param(&key[..], s.clone()),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            q.param(&key[..], i)
                        } else if let Some(f) = n.as_f64() {
                            q.param(&key[..], f)
                        } else {
                            q.param(&key[..], n.to_string())
                        }
                    }
                    serde_json::Value::Bool(b) => q.param(&key[..], *b),
                    serde_json::Value::Null => q.param(&key[..], ""),
                    _ => q.param(&key[..], value.to_string()),
                };
            }
        }

        let mut stream = timed(self.graph()?.execute(q))
            .await?
            .map_err(|e| ArgusError::Graph(format!("Failed to execute cypher: {}", e)))?;

        let mut rows = Vec::new();
        while let Ok(Some(row)) = stream.next().await {
            // Attempt to serialize the row to JSON by extracting known column patterns
            // neo4rs Row doesn't directly serialize, so we extract what we can
            let row_json: serde_json::Value = row
                .to()
                .unwrap_or(serde_json::Value::Object(Default::default()));
            rows.push(row_json);
        }

        tracing::debug!(
            cypher = %graph_query.cypher,
            rows = rows.len(),
            "Executed raw Cypher query"
        );

        Ok(serde_json::Value::Array(rows))
    }

    async fn entity_count(&self) -> Result<u64> {
        let mut stream = timed(
            self.graph()?
                .execute(query("MATCH (n) RETURN count(n) AS cnt")),
        )
        .await?
        .map_err(|e| ArgusError::Graph(format!("Failed to count entities: {}", e)))?;

        match stream.next().await {
            Ok(Some(row)) => {
                let count: i64 = row
                    .get("cnt")
                    .map_err(|e| ArgusError::Graph(format!("Failed to get count: {}", e)))?;
                Ok(count as u64)
            }
            Ok(None) => Ok(0),
            Err(e) => Err(ArgusError::Graph(format!("Error counting entities: {}", e))),
        }
    }

    async fn relationship_count(&self) -> Result<u64> {
        let mut stream = timed(
            self.graph()?
                .execute(query("MATCH ()-[r]->() RETURN count(r) AS cnt")),
        )
        .await?
        .map_err(|e| ArgusError::Graph(format!("Failed to count relationships: {}", e)))?;

        match stream.next().await {
            Ok(Some(row)) => {
                let count: i64 = row
                    .get("cnt")
                    .map_err(|e| ArgusError::Graph(format!("Failed to get count: {}", e)))?;
                Ok(count as u64)
            }
            Ok(None) => Ok(0),
            Err(e) => Err(ArgusError::Graph(format!(
                "Error counting relationships: {}",
                e
            ))),
        }
    }
}
