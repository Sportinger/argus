use argus_core::api_types::{
    AgentListResponse, AgentTriggerRequest, AgentTriggerResponse, EntityDetailResponse,
    EntitySearchRequest, EntitySearchResponse, EntityTypeStat, GraphQueryRequest,
    GraphQueryResponse, GraphStatsResponse, HealthResponse, ReasoningApiResponse,
    ReasoningRequest, TimelineEvent, TimelineRequest, TimelineResponse,
};
use argus_core::agent::AgentStatus;
use argus_core::config::AppConfig;
use argus_core::entity::{Entity, EntityType, Relationship, RelationType};
use argus_core::reasoning::{ReasoningResponse, ReasoningStep};
use chrono::Utc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// HealthResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn health_response_roundtrip() {
    let hr = HealthResponse {
        status: "ok".to_string(),
        version: "0.1.0".to_string(),
        neo4j_connected: true,
        qdrant_connected: false,
        entity_count: 1000,
        relationship_count: 5000,
    };

    let json = serde_json::to_string(&hr).expect("failed to serialize HealthResponse");
    let deserialized: HealthResponse =
        serde_json::from_str(&json).expect("failed to deserialize HealthResponse");

    assert_eq!(deserialized.status, "ok");
    assert_eq!(deserialized.version, "0.1.0");
    assert!(deserialized.neo4j_connected);
    assert!(!deserialized.qdrant_connected);
    assert_eq!(deserialized.entity_count, 1000);
    assert_eq!(deserialized.relationship_count, 5000);
}

// ---------------------------------------------------------------------------
// AgentListResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn agent_list_response_roundtrip() {
    let response = AgentListResponse {
        agents: vec![
            AgentStatus {
                name: "gdelt".to_string(),
                enabled: true,
                last_run: None,
                documents_collected: 0,
                error: None,
            },
            AgentStatus {
                name: "adsb".to_string(),
                enabled: false,
                last_run: Some(Utc::now()),
                documents_collected: 42,
                error: Some("timeout".to_string()),
            },
        ],
    };

    let json = serde_json::to_string(&response).expect("failed to serialize AgentListResponse");
    let deserialized: AgentListResponse =
        serde_json::from_str(&json).expect("failed to deserialize AgentListResponse");

    assert_eq!(deserialized.agents.len(), 2);
    assert_eq!(deserialized.agents[0].name, "gdelt");
    assert!(deserialized.agents[0].enabled);
    assert_eq!(deserialized.agents[1].name, "adsb");
    assert!(!deserialized.agents[1].enabled);
    assert_eq!(deserialized.agents[1].documents_collected, 42);
    assert_eq!(deserialized.agents[1].error.as_deref(), Some("timeout"));
}

#[test]
fn agent_list_response_empty() {
    let response = AgentListResponse { agents: vec![] };

    let json = serde_json::to_string(&response).unwrap();
    let deserialized: AgentListResponse = serde_json::from_str(&json).unwrap();

    assert!(deserialized.agents.is_empty());
}

// ---------------------------------------------------------------------------
// AgentTriggerRequest serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn agent_trigger_request_roundtrip() {
    let req = AgentTriggerRequest {
        agent_name: "gdelt".to_string(),
    };

    let json = serde_json::to_string(&req).expect("failed to serialize AgentTriggerRequest");
    let deserialized: AgentTriggerRequest =
        serde_json::from_str(&json).expect("failed to deserialize AgentTriggerRequest");

    assert_eq!(deserialized.agent_name, "gdelt");
}

// ---------------------------------------------------------------------------
// AgentTriggerResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn agent_trigger_response_roundtrip() {
    let resp = AgentTriggerResponse {
        agent_name: "opensanctions".to_string(),
        documents_collected: 150,
        message: "Collection completed successfully".to_string(),
    };

    let json = serde_json::to_string(&resp).expect("failed to serialize AgentTriggerResponse");
    let deserialized: AgentTriggerResponse =
        serde_json::from_str(&json).expect("failed to deserialize AgentTriggerResponse");

    assert_eq!(deserialized.agent_name, "opensanctions");
    assert_eq!(deserialized.documents_collected, 150);
    assert_eq!(deserialized.message, "Collection completed successfully");
}

// ---------------------------------------------------------------------------
// EntitySearchRequest serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn entity_search_request_roundtrip() {
    let req = EntitySearchRequest {
        query: "John Doe".to_string(),
        limit: 10,
        entity_type: Some(EntityType::Person),
    };

    let json = serde_json::to_string(&req).expect("failed to serialize EntitySearchRequest");
    let deserialized: EntitySearchRequest =
        serde_json::from_str(&json).expect("failed to deserialize EntitySearchRequest");

    assert_eq!(deserialized.query, "John Doe");
    assert_eq!(deserialized.limit, 10);
    assert_eq!(deserialized.entity_type, Some(EntityType::Person));
}

#[test]
fn entity_search_request_default_limit() {
    // When limit is not provided in JSON, it should default to 20
    let json = r#"{"query": "test"}"#;
    let deserialized: EntitySearchRequest =
        serde_json::from_str(json).expect("failed to deserialize EntitySearchRequest with default");

    assert_eq!(deserialized.query, "test");
    assert_eq!(deserialized.limit, 20);
    assert!(deserialized.entity_type.is_none());
}

#[test]
fn entity_search_request_without_entity_type() {
    let req = EntitySearchRequest {
        query: "search term".to_string(),
        limit: 50,
        entity_type: None,
    };

    let json = serde_json::to_string(&req).unwrap();
    let deserialized: EntitySearchRequest = serde_json::from_str(&json).unwrap();

    assert!(deserialized.entity_type.is_none());
    assert_eq!(deserialized.limit, 50);
}

// ---------------------------------------------------------------------------
// EntitySearchResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn entity_search_response_roundtrip() {
    let entity = Entity::new(
        EntityType::Person,
        "Jane Smith".to_string(),
        "test".to_string(),
    );

    let resp = EntitySearchResponse {
        entities: vec![entity],
        total: 1,
    };

    let json = serde_json::to_string(&resp).expect("failed to serialize EntitySearchResponse");
    let deserialized: EntitySearchResponse =
        serde_json::from_str(&json).expect("failed to deserialize EntitySearchResponse");

    assert_eq!(deserialized.entities.len(), 1);
    assert_eq!(deserialized.entities[0].name, "Jane Smith");
    assert_eq!(deserialized.total, 1);
}

#[test]
fn entity_search_response_empty() {
    let resp = EntitySearchResponse {
        entities: vec![],
        total: 0,
    };

    let json = serde_json::to_string(&resp).unwrap();
    let deserialized: EntitySearchResponse = serde_json::from_str(&json).unwrap();

    assert!(deserialized.entities.is_empty());
    assert_eq!(deserialized.total, 0);
}

// ---------------------------------------------------------------------------
// GraphQueryRequest serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn graph_query_request_roundtrip() {
    let req = GraphQueryRequest {
        cypher: "MATCH (n:Person) RETURN n LIMIT 10".to_string(),
        params: serde_json::json!({"name": "John"}),
    };

    let json = serde_json::to_string(&req).expect("failed to serialize GraphQueryRequest");
    let deserialized: GraphQueryRequest =
        serde_json::from_str(&json).expect("failed to deserialize GraphQueryRequest");

    assert_eq!(deserialized.cypher, "MATCH (n:Person) RETURN n LIMIT 10");
    assert_eq!(deserialized.params["name"], "John");
}

#[test]
fn graph_query_request_default_params() {
    // When params is not provided, it should default to null (serde default)
    let json = r#"{"cypher": "MATCH (n) RETURN count(n)"}"#;
    let deserialized: GraphQueryRequest =
        serde_json::from_str(json).expect("failed to deserialize GraphQueryRequest with default params");

    assert_eq!(deserialized.cypher, "MATCH (n) RETURN count(n)");
    // serde(default) on serde_json::Value yields Value::Null
    assert!(deserialized.params.is_null());
}

// ---------------------------------------------------------------------------
// GraphQueryResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn graph_query_response_roundtrip() {
    let resp = GraphQueryResponse {
        result: serde_json::json!({"count": 42, "data": [1, 2, 3]}),
    };

    let json = serde_json::to_string(&resp).expect("failed to serialize GraphQueryResponse");
    let deserialized: GraphQueryResponse =
        serde_json::from_str(&json).expect("failed to deserialize GraphQueryResponse");

    assert_eq!(deserialized.result["count"], 42);
    assert_eq!(deserialized.result["data"][0], 1);
}

// ---------------------------------------------------------------------------
// GraphStatsResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn graph_stats_response_roundtrip() {
    let resp = GraphStatsResponse {
        entity_count: 5000,
        relationship_count: 12000,
        entity_types: vec![
            EntityTypeStat {
                entity_type: EntityType::Person,
                count: 2000,
            },
            EntityTypeStat {
                entity_type: EntityType::Organization,
                count: 1500,
            },
            EntityTypeStat {
                entity_type: EntityType::Vessel,
                count: 500,
            },
        ],
    };

    let json = serde_json::to_string(&resp).expect("failed to serialize GraphStatsResponse");
    let deserialized: GraphStatsResponse =
        serde_json::from_str(&json).expect("failed to deserialize GraphStatsResponse");

    assert_eq!(deserialized.entity_count, 5000);
    assert_eq!(deserialized.relationship_count, 12000);
    assert_eq!(deserialized.entity_types.len(), 3);
    assert_eq!(deserialized.entity_types[0].entity_type, EntityType::Person);
    assert_eq!(deserialized.entity_types[0].count, 2000);
    assert_eq!(
        deserialized.entity_types[1].entity_type,
        EntityType::Organization
    );
    assert_eq!(deserialized.entity_types[2].count, 500);
}

#[test]
fn graph_stats_response_empty_entity_types() {
    let resp = GraphStatsResponse {
        entity_count: 0,
        relationship_count: 0,
        entity_types: vec![],
    };

    let json = serde_json::to_string(&resp).unwrap();
    let deserialized: GraphStatsResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.entity_count, 0);
    assert!(deserialized.entity_types.is_empty());
}

// ---------------------------------------------------------------------------
// ReasoningRequest serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn reasoning_request_roundtrip() {
    let req = ReasoningRequest {
        question: "Who owns ACME Corp?".to_string(),
        context: Some("Corporate ownership analysis".to_string()),
        max_hops: Some(3),
    };

    let json = serde_json::to_string(&req).expect("failed to serialize ReasoningRequest");
    let deserialized: ReasoningRequest =
        serde_json::from_str(&json).expect("failed to deserialize ReasoningRequest");

    assert_eq!(deserialized.question, "Who owns ACME Corp?");
    assert_eq!(
        deserialized.context.as_deref(),
        Some("Corporate ownership analysis")
    );
    assert_eq!(deserialized.max_hops, Some(3));
}

#[test]
fn reasoning_request_minimal() {
    let json = r#"{"question": "What is going on?"}"#;
    let deserialized: ReasoningRequest =
        serde_json::from_str(json).expect("failed to deserialize minimal ReasoningRequest");

    assert_eq!(deserialized.question, "What is going on?");
    assert!(deserialized.context.is_none());
    assert!(deserialized.max_hops.is_none());
}

// ---------------------------------------------------------------------------
// ReasoningApiResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn reasoning_api_response_roundtrip() {
    let entity = Entity::new(
        EntityType::Person,
        "Bob".to_string(),
        "reasoning".to_string(),
    );

    let resp = ReasoningApiResponse {
        answer: "Bob owns ACME Corp through a subsidiary.".to_string(),
        confidence: 0.85,
        steps: vec![
            ReasoningStep {
                description: "Search for ACME Corp ownership".to_string(),
                cypher: Some("MATCH (p)-[:OWNER_OF]->(o) WHERE o.name = 'ACME' RETURN p".to_string()),
                result_summary: "Found 1 owner".to_string(),
            },
            ReasoningStep {
                description: "Verify relationship".to_string(),
                cypher: None,
                result_summary: "Confirmed ownership chain".to_string(),
            },
        ],
        entities_referenced: vec![entity],
        sources: vec![
            "opencorporates".to_string(),
            "opensanctions".to_string(),
        ],
    };

    let json =
        serde_json::to_string(&resp).expect("failed to serialize ReasoningApiResponse");
    let deserialized: ReasoningApiResponse =
        serde_json::from_str(&json).expect("failed to deserialize ReasoningApiResponse");

    assert_eq!(
        deserialized.answer,
        "Bob owns ACME Corp through a subsidiary."
    );
    assert_eq!(deserialized.confidence, 0.85);
    assert_eq!(deserialized.steps.len(), 2);
    assert_eq!(
        deserialized.steps[0].description,
        "Search for ACME Corp ownership"
    );
    assert!(deserialized.steps[0].cypher.is_some());
    assert!(deserialized.steps[1].cypher.is_none());
    assert_eq!(deserialized.entities_referenced.len(), 1);
    assert_eq!(deserialized.entities_referenced[0].name, "Bob");
    assert_eq!(deserialized.sources.len(), 2);
}

// ---------------------------------------------------------------------------
// ReasoningApiResponse From<ReasoningResponse> conversion
// ---------------------------------------------------------------------------

#[test]
fn reasoning_api_response_from_reasoning_response() {
    let entity = Entity::new(
        EntityType::Organization,
        "Shell Corp".to_string(),
        "test".to_string(),
    );
    let entity_id = entity.id;

    let reasoning_response = ReasoningResponse {
        answer: "Shell Corp is sanctioned.".to_string(),
        confidence: 0.95,
        steps: vec![ReasoningStep {
            description: "Lookup sanctions database".to_string(),
            cypher: Some("MATCH (e)-[:SANCTIONED_BY]->(s) RETURN e, s".to_string()),
            result_summary: "Found in OFAC SDN list".to_string(),
        }],
        entities_referenced: vec![entity],
        sources: vec!["opensanctions".to_string()],
    };

    let api_response: ReasoningApiResponse = reasoning_response.into();

    assert_eq!(api_response.answer, "Shell Corp is sanctioned.");
    assert_eq!(api_response.confidence, 0.95);
    assert_eq!(api_response.steps.len(), 1);
    assert_eq!(
        api_response.steps[0].description,
        "Lookup sanctions database"
    );
    assert_eq!(api_response.entities_referenced.len(), 1);
    assert_eq!(api_response.entities_referenced[0].id, entity_id);
    assert_eq!(api_response.entities_referenced[0].name, "Shell Corp");
    assert_eq!(api_response.sources, vec!["opensanctions"]);
}

#[test]
fn reasoning_api_response_from_empty_reasoning_response() {
    let reasoning_response = ReasoningResponse {
        answer: "No relevant information found.".to_string(),
        confidence: 0.0,
        steps: vec![],
        entities_referenced: vec![],
        sources: vec![],
    };

    let api_response: ReasoningApiResponse = reasoning_response.into();

    assert_eq!(api_response.answer, "No relevant information found.");
    assert_eq!(api_response.confidence, 0.0);
    assert!(api_response.steps.is_empty());
    assert!(api_response.entities_referenced.is_empty());
    assert!(api_response.sources.is_empty());
}

// ---------------------------------------------------------------------------
// TimelineRequest serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn timeline_request_roundtrip() {
    let entity_id = Uuid::new_v4();
    let now = Utc::now();
    let req = TimelineRequest {
        entity_id: Some(entity_id),
        start: Some(now),
        end: None,
        limit: 50,
    };

    let json = serde_json::to_string(&req).expect("failed to serialize TimelineRequest");
    let deserialized: TimelineRequest =
        serde_json::from_str(&json).expect("failed to deserialize TimelineRequest");

    assert_eq!(deserialized.entity_id, Some(entity_id));
    assert!(deserialized.start.is_some());
    assert!(deserialized.end.is_none());
    assert_eq!(deserialized.limit, 50);
}

#[test]
fn timeline_request_default_limit() {
    let json = r#"{}"#;
    let deserialized: TimelineRequest =
        serde_json::from_str(json).expect("failed to deserialize empty TimelineRequest");

    assert!(deserialized.entity_id.is_none());
    assert!(deserialized.start.is_none());
    assert!(deserialized.end.is_none());
    assert_eq!(deserialized.limit, 20);
}

// ---------------------------------------------------------------------------
// TimelineResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn timeline_response_roundtrip() {
    let entity = Entity::new(
        EntityType::Event,
        "UN Summit".to_string(),
        "gdelt".to_string(),
    );

    let resp = TimelineResponse {
        events: vec![TimelineEvent {
            timestamp: Utc::now(),
            entity,
            event_type: "diplomatic_meeting".to_string(),
            description: "UN General Assembly session".to_string(),
            source: "gdelt".to_string(),
        }],
    };

    let json = serde_json::to_string(&resp).expect("failed to serialize TimelineResponse");
    let deserialized: TimelineResponse =
        serde_json::from_str(&json).expect("failed to deserialize TimelineResponse");

    assert_eq!(deserialized.events.len(), 1);
    assert_eq!(deserialized.events[0].event_type, "diplomatic_meeting");
    assert_eq!(
        deserialized.events[0].description,
        "UN General Assembly session"
    );
    assert_eq!(deserialized.events[0].source, "gdelt");
    assert_eq!(deserialized.events[0].entity.name, "UN Summit");
}

#[test]
fn timeline_response_empty() {
    let resp = TimelineResponse { events: vec![] };

    let json = serde_json::to_string(&resp).unwrap();
    let deserialized: TimelineResponse = serde_json::from_str(&json).unwrap();

    assert!(deserialized.events.is_empty());
}

// ---------------------------------------------------------------------------
// EntityDetailResponse serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn entity_detail_response_roundtrip() {
    let entity = Entity::new(
        EntityType::Person,
        "Alice".to_string(),
        "test".to_string(),
    );
    let neighbor = Entity::new(
        EntityType::Organization,
        "Corp Inc".to_string(),
        "test".to_string(),
    );
    let rel = Relationship::new(
        entity.id,
        neighbor.id,
        RelationType::EmployeeOf,
        "test".to_string(),
    );

    let resp = EntityDetailResponse {
        entity: entity.clone(),
        relationships: vec![rel],
        neighbors: vec![neighbor],
    };

    let json = serde_json::to_string(&resp).expect("failed to serialize EntityDetailResponse");
    let deserialized: EntityDetailResponse =
        serde_json::from_str(&json).expect("failed to deserialize EntityDetailResponse");

    assert_eq!(deserialized.entity.id, entity.id);
    assert_eq!(deserialized.entity.name, "Alice");
    assert_eq!(deserialized.relationships.len(), 1);
    assert_eq!(
        deserialized.relationships[0].relation_type,
        RelationType::EmployeeOf
    );
    assert_eq!(deserialized.neighbors.len(), 1);
    assert_eq!(deserialized.neighbors[0].name, "Corp Inc");
}

// ---------------------------------------------------------------------------
// EntityTypeStat serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn entity_type_stat_roundtrip() {
    let stat = EntityTypeStat {
        entity_type: EntityType::Sanction,
        count: 9999,
    };

    let json = serde_json::to_string(&stat).expect("failed to serialize EntityTypeStat");
    let deserialized: EntityTypeStat =
        serde_json::from_str(&json).expect("failed to deserialize EntityTypeStat");

    assert_eq!(deserialized.entity_type, EntityType::Sanction);
    assert_eq!(deserialized.count, 9999);
}

// ---------------------------------------------------------------------------
// AppConfig::from_env() with default values
// ---------------------------------------------------------------------------

#[test]
fn app_config_from_env_defaults() {
    // Clear any existing env vars that could interfere
    std::env::remove_var("NEO4J_URI");
    std::env::remove_var("NEO4J_USER");
    std::env::remove_var("NEO4J_PASSWORD");
    std::env::remove_var("QDRANT_URL");
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("SERVER_HOST");
    std::env::remove_var("SERVER_PORT");

    let config = AppConfig::from_env();

    assert_eq!(config.neo4j_uri, "bolt://localhost:7687");
    assert_eq!(config.neo4j_user, "neo4j");
    assert_eq!(config.neo4j_password, "argus");
    assert_eq!(config.qdrant_url, "http://localhost:6333");
    assert_eq!(config.anthropic_api_key, ""); // unwrap_or_default
    assert_eq!(config.server_host, "0.0.0.0");
    assert_eq!(config.server_port, 8080);
    assert!(config.sources.is_empty());
}

#[test]
fn app_config_from_env_custom_values() {
    // Set custom env vars
    std::env::set_var("NEO4J_URI", "bolt://custom:7688");
    std::env::set_var("NEO4J_USER", "admin");
    std::env::set_var("NEO4J_PASSWORD", "secret");
    std::env::set_var("QDRANT_URL", "http://qdrant:6334");
    std::env::set_var("ANTHROPIC_API_KEY", "sk-test-key");
    std::env::set_var("SERVER_HOST", "127.0.0.1");
    std::env::set_var("SERVER_PORT", "3000");

    let config = AppConfig::from_env();

    assert_eq!(config.neo4j_uri, "bolt://custom:7688");
    assert_eq!(config.neo4j_user, "admin");
    assert_eq!(config.neo4j_password, "secret");
    assert_eq!(config.qdrant_url, "http://qdrant:6334");
    assert_eq!(config.anthropic_api_key, "sk-test-key");
    assert_eq!(config.server_host, "127.0.0.1");
    assert_eq!(config.server_port, 3000);

    // Clean up
    std::env::remove_var("NEO4J_URI");
    std::env::remove_var("NEO4J_USER");
    std::env::remove_var("NEO4J_PASSWORD");
    std::env::remove_var("QDRANT_URL");
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("SERVER_HOST");
    std::env::remove_var("SERVER_PORT");
}

#[test]
fn app_config_from_env_invalid_port_falls_back_to_default() {
    std::env::set_var("SERVER_PORT", "not_a_number");

    let config = AppConfig::from_env();
    assert_eq!(config.server_port, 8080);

    std::env::remove_var("SERVER_PORT");
}

#[test]
fn app_config_serialization_roundtrip() {
    let config = AppConfig {
        neo4j_uri: "bolt://localhost:7687".to_string(),
        neo4j_user: "neo4j".to_string(),
        neo4j_password: "pass".to_string(),
        qdrant_url: "http://localhost:6333".to_string(),
        anthropic_api_key: "key".to_string(),
        server_host: "0.0.0.0".to_string(),
        server_port: 8080,
        sources: vec![],
    };

    let json = serde_json::to_string(&config).expect("failed to serialize AppConfig");
    let deserialized: AppConfig =
        serde_json::from_str(&json).expect("failed to deserialize AppConfig");

    assert_eq!(deserialized.neo4j_uri, config.neo4j_uri);
    assert_eq!(deserialized.server_port, config.server_port);
    assert!(deserialized.sources.is_empty());
}

// ---------------------------------------------------------------------------
// default_limit values (verified through EntitySearchRequest and TimelineRequest)
// ---------------------------------------------------------------------------

#[test]
fn default_limit_is_20_for_entity_search() {
    let json = r#"{"query": "test"}"#;
    let req: EntitySearchRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.limit, 20);
}

#[test]
fn default_limit_is_20_for_timeline() {
    let json = r#"{}"#;
    let req: TimelineRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.limit, 20);
}

#[test]
fn explicit_limit_overrides_default_for_entity_search() {
    let json = r#"{"query": "test", "limit": 100}"#;
    let req: EntitySearchRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.limit, 100);
}

#[test]
fn explicit_limit_overrides_default_for_timeline() {
    let json = r#"{"limit": 5}"#;
    let req: TimelineRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.limit, 5);
}

// ---------------------------------------------------------------------------
// ReasoningStep serialization
// ---------------------------------------------------------------------------

#[test]
fn reasoning_step_roundtrip() {
    let step = ReasoningStep {
        description: "Query the graph".to_string(),
        cypher: Some("MATCH (n) RETURN n".to_string()),
        result_summary: "Found 10 nodes".to_string(),
    };

    let json = serde_json::to_string(&step).expect("failed to serialize ReasoningStep");
    let deserialized: ReasoningStep =
        serde_json::from_str(&json).expect("failed to deserialize ReasoningStep");

    assert_eq!(deserialized.description, "Query the graph");
    assert_eq!(deserialized.cypher, Some("MATCH (n) RETURN n".to_string()));
    assert_eq!(deserialized.result_summary, "Found 10 nodes");
}

#[test]
fn reasoning_step_without_cypher() {
    let step = ReasoningStep {
        description: "Analyze results".to_string(),
        cypher: None,
        result_summary: "Confirmed pattern".to_string(),
    };

    let json = serde_json::to_string(&step).unwrap();
    let deserialized: ReasoningStep = serde_json::from_str(&json).unwrap();

    assert!(deserialized.cypher.is_none());
}

// ---------------------------------------------------------------------------
// Cross-type integration: complex nested structures
// ---------------------------------------------------------------------------

#[test]
fn full_graph_stats_with_all_entity_types() {
    let all_types = vec![
        EntityType::Person,
        EntityType::Organization,
        EntityType::Vessel,
        EntityType::Aircraft,
        EntityType::Location,
        EntityType::Event,
        EntityType::Document,
        EntityType::Transaction,
        EntityType::Sanction,
    ];

    let stats: Vec<EntityTypeStat> = all_types
        .into_iter()
        .enumerate()
        .map(|(i, et)| EntityTypeStat {
            entity_type: et,
            count: (i as u64 + 1) * 100,
        })
        .collect();

    let resp = GraphStatsResponse {
        entity_count: stats.iter().map(|s| s.count).sum(),
        relationship_count: 50000,
        entity_types: stats,
    };

    let json = serde_json::to_string(&resp).unwrap();
    let deserialized: GraphStatsResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.entity_types.len(), 9);
    assert_eq!(deserialized.entity_count, 4500); // sum of 100+200+...+900
    assert_eq!(deserialized.entity_types[0].count, 100);
    assert_eq!(deserialized.entity_types[8].count, 900);
}
