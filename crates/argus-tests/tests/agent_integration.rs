use argus_agents::agent_registry;
use argus_core::agent::{AgentStatus, RawDocument};
use argus_core::entity::{Entity, EntityType, ExtractionResult, RelationType, Relationship};
use argus_core::Agent;
use chrono::Utc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// agent_registry tests
// ---------------------------------------------------------------------------

#[test]
fn agent_registry_returns_all_six_agents() {
    let registry = agent_registry();
    assert_eq!(registry.len(), 6);
}

#[test]
fn agent_registry_contains_expected_keys() {
    let registry = agent_registry();
    let expected_keys = ["gdelt", "opencorporates", "ais", "adsb", "opensanctions", "eu_transparency"];
    for key in &expected_keys {
        assert!(
            registry.contains_key(*key),
            "registry missing expected key: {key}"
        );
    }
}

// ---------------------------------------------------------------------------
// Agent::name() and Agent::source_type() for each agent
// ---------------------------------------------------------------------------

#[test]
fn gdelt_agent_name_and_source_type() {
    let registry = agent_registry();
    let agent = registry.get("gdelt").expect("gdelt agent not found");
    assert_eq!(agent.name(), "gdelt");
    assert_eq!(agent.source_type(), "news_events");
}

#[test]
fn opencorporates_agent_name_and_source_type() {
    let registry = agent_registry();
    let agent = registry.get("opencorporates").expect("opencorporates agent not found");
    assert_eq!(agent.name(), "opencorporates");
    assert_eq!(agent.source_type(), "corporate_registry");
}

#[test]
fn ais_agent_name_and_source_type() {
    let registry = agent_registry();
    let agent = registry.get("ais").expect("ais agent not found");
    assert_eq!(agent.name(), "ais");
    assert_eq!(agent.source_type(), "maritime_tracking");
}

#[test]
fn adsb_agent_name_and_source_type() {
    let registry = agent_registry();
    let agent = registry.get("adsb").expect("adsb agent not found");
    assert_eq!(agent.name(), "adsb");
    assert_eq!(agent.source_type(), "aircraft_tracking");
}

#[test]
fn opensanctions_agent_name_and_source_type() {
    let registry = agent_registry();
    let agent = registry.get("opensanctions").expect("opensanctions agent not found");
    assert_eq!(agent.name(), "opensanctions");
    assert_eq!(agent.source_type(), "sanctions");
}

#[test]
fn eu_transparency_agent_name_and_source_type() {
    let registry = agent_registry();
    let agent = registry.get("eu_transparency").expect("eu_transparency agent not found");
    assert_eq!(agent.name(), "eu_transparency");
    assert_eq!(agent.source_type(), "lobby_register");
}

// ---------------------------------------------------------------------------
// Agent names from registry match their keys
// ---------------------------------------------------------------------------

#[test]
fn agent_names_match_registry_keys() {
    let registry = agent_registry();
    for (key, agent) in &registry {
        assert_eq!(
            key,
            agent.name(),
            "registry key '{key}' does not match agent.name() '{}'",
            agent.name()
        );
    }
}

// ---------------------------------------------------------------------------
// AgentStatus fields
// ---------------------------------------------------------------------------

#[test]
fn agent_status_fields_properly_initialized() {
    let status = AgentStatus {
        name: "test_agent".to_string(),
        enabled: true,
        last_run: None,
        documents_collected: 0,
        error: None,
    };

    assert_eq!(status.name, "test_agent");
    assert!(status.enabled);
    assert!(status.last_run.is_none());
    assert_eq!(status.documents_collected, 0);
    assert!(status.error.is_none());
}

#[test]
fn agent_status_with_last_run_and_error() {
    let now = Utc::now();
    let status = AgentStatus {
        name: "failing_agent".to_string(),
        enabled: false,
        last_run: Some(now),
        documents_collected: 42,
        error: Some("connection timeout".to_string()),
    };

    assert_eq!(status.name, "failing_agent");
    assert!(!status.enabled);
    assert_eq!(status.last_run, Some(now));
    assert_eq!(status.documents_collected, 42);
    assert_eq!(status.error.as_deref(), Some("connection timeout"));
}

#[test]
fn agent_status_serialization_roundtrip() {
    let now = Utc::now();
    let status = AgentStatus {
        name: "test".to_string(),
        enabled: true,
        last_run: Some(now),
        documents_collected: 100,
        error: None,
    };

    let json = serde_json::to_string(&status).expect("failed to serialize AgentStatus");
    let deserialized: AgentStatus =
        serde_json::from_str(&json).expect("failed to deserialize AgentStatus");

    assert_eq!(deserialized.name, status.name);
    assert_eq!(deserialized.enabled, status.enabled);
    assert_eq!(deserialized.documents_collected, status.documents_collected);
    assert_eq!(deserialized.error, status.error);
}

// ---------------------------------------------------------------------------
// AgentStatus via async status() calls on each agent
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_agents_initial_status_is_clean() {
    let registry = agent_registry();
    for (key, agent) in &registry {
        let status = agent.status().await;
        assert_eq!(
            status.name, *key,
            "agent '{key}' status.name does not match"
        );
        assert!(
            status.enabled,
            "agent '{key}' should be enabled by default"
        );
        assert!(
            status.last_run.is_none(),
            "agent '{key}' should have no last_run initially"
        );
        assert_eq!(
            status.documents_collected, 0,
            "agent '{key}' should have 0 documents initially"
        );
        assert!(
            status.error.is_none(),
            "agent '{key}' should have no error initially"
        );
    }
}

// ---------------------------------------------------------------------------
// RawDocument creation and serialization
// ---------------------------------------------------------------------------

#[test]
fn raw_document_creation() {
    let now = Utc::now();
    let doc = RawDocument {
        source: "test_source".to_string(),
        source_id: "doc-001".to_string(),
        title: Some("Test Document".to_string()),
        content: "This is the content of the test document.".to_string(),
        url: Some("https://example.com/doc/001".to_string()),
        collected_at: now,
        metadata: serde_json::json!({"key": "value", "count": 42}),
    };

    assert_eq!(doc.source, "test_source");
    assert_eq!(doc.source_id, "doc-001");
    assert_eq!(doc.title, Some("Test Document".to_string()));
    assert_eq!(doc.content, "This is the content of the test document.");
    assert_eq!(doc.url, Some("https://example.com/doc/001".to_string()));
    assert_eq!(doc.collected_at, now);
    assert_eq!(doc.metadata["key"], "value");
    assert_eq!(doc.metadata["count"], 42);
}

#[test]
fn raw_document_with_none_fields() {
    let doc = RawDocument {
        source: "minimal".to_string(),
        source_id: "min-001".to_string(),
        title: None,
        content: "content".to_string(),
        url: None,
        collected_at: Utc::now(),
        metadata: serde_json::json!({}),
    };

    assert!(doc.title.is_none());
    assert!(doc.url.is_none());
}

#[test]
fn raw_document_serialization_roundtrip() {
    let now = Utc::now();
    let doc = RawDocument {
        source: "test".to_string(),
        source_id: "id-123".to_string(),
        title: Some("A Title".to_string()),
        content: "Some content here.".to_string(),
        url: Some("https://example.com".to_string()),
        collected_at: now,
        metadata: serde_json::json!({"nested": {"a": 1}}),
    };

    let json = serde_json::to_string(&doc).expect("failed to serialize RawDocument");
    let deserialized: RawDocument =
        serde_json::from_str(&json).expect("failed to deserialize RawDocument");

    assert_eq!(deserialized.source, doc.source);
    assert_eq!(deserialized.source_id, doc.source_id);
    assert_eq!(deserialized.title, doc.title);
    assert_eq!(deserialized.content, doc.content);
    assert_eq!(deserialized.url, doc.url);
    assert_eq!(deserialized.metadata, doc.metadata);
}

// ---------------------------------------------------------------------------
// Entity creation with Entity::new()
// ---------------------------------------------------------------------------

#[test]
fn entity_new_sets_defaults() {
    let entity = Entity::new(
        EntityType::Person,
        "John Doe".to_string(),
        "test".to_string(),
    );

    assert_eq!(entity.entity_type, EntityType::Person);
    assert_eq!(entity.name, "John Doe");
    assert_eq!(entity.source, "test");
    assert!(entity.aliases.is_empty());
    assert_eq!(
        entity.properties,
        serde_json::Value::Object(serde_json::Map::new())
    );
    assert!(entity.source_id.is_none());
    assert_eq!(entity.confidence, 1.0);
    // first_seen and last_seen should be equal (both set to now)
    assert_eq!(entity.first_seen, entity.last_seen);
    // id should be a valid UUID
    assert!(!entity.id.is_nil());
}

#[test]
fn entity_new_generates_unique_ids() {
    let e1 = Entity::new(EntityType::Person, "A".to_string(), "src".to_string());
    let e2 = Entity::new(EntityType::Person, "B".to_string(), "src".to_string());
    assert_ne!(e1.id, e2.id);
}

#[test]
fn entity_new_with_different_types() {
    let types_and_names = vec![
        (EntityType::Person, "Alice"),
        (EntityType::Organization, "ACME Corp"),
        (EntityType::Vessel, "SS Enterprise"),
        (EntityType::Aircraft, "Boeing 747"),
        (EntityType::Location, "New York"),
        (EntityType::Event, "Summit 2026"),
        (EntityType::Document, "Report #42"),
        (EntityType::Transaction, "TX-001"),
        (EntityType::Sanction, "OFAC-12345"),
    ];

    for (entity_type, name) in types_and_names {
        let entity = Entity::new(entity_type.clone(), name.to_string(), "test".to_string());
        assert_eq!(entity.entity_type, entity_type);
        assert_eq!(entity.name, name);
    }
}

#[test]
fn entity_serialization_roundtrip() {
    let entity = Entity::new(
        EntityType::Organization,
        "Test Corp".to_string(),
        "opensanctions".to_string(),
    );

    let json = serde_json::to_string(&entity).expect("failed to serialize Entity");
    let deserialized: Entity =
        serde_json::from_str(&json).expect("failed to deserialize Entity");

    assert_eq!(deserialized.id, entity.id);
    assert_eq!(deserialized.entity_type, entity.entity_type);
    assert_eq!(deserialized.name, entity.name);
    assert_eq!(deserialized.source, entity.source);
    assert_eq!(deserialized.confidence, entity.confidence);
}

// ---------------------------------------------------------------------------
// Relationship creation with Relationship::new()
// ---------------------------------------------------------------------------

#[test]
fn relationship_new_sets_defaults() {
    let source_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();

    let rel = Relationship::new(
        source_id,
        target_id,
        RelationType::OwnerOf,
        "test".to_string(),
    );

    assert_eq!(rel.source_entity_id, source_id);
    assert_eq!(rel.target_entity_id, target_id);
    assert_eq!(rel.relation_type, RelationType::OwnerOf);
    assert_eq!(rel.source, "test");
    assert_eq!(
        rel.properties,
        serde_json::Value::Object(serde_json::Map::new())
    );
    assert_eq!(rel.confidence, 1.0);
    assert!(rel.timestamp.is_none());
    assert!(!rel.id.is_nil());
}

#[test]
fn relationship_new_generates_unique_ids() {
    let src = Uuid::new_v4();
    let tgt = Uuid::new_v4();
    let r1 = Relationship::new(src, tgt, RelationType::RelatedTo, "a".to_string());
    let r2 = Relationship::new(src, tgt, RelationType::RelatedTo, "b".to_string());
    assert_ne!(r1.id, r2.id);
}

#[test]
fn relationship_new_with_all_relation_types() {
    let src = Uuid::new_v4();
    let tgt = Uuid::new_v4();

    let types = vec![
        RelationType::OwnerOf,
        RelationType::DirectorOf,
        RelationType::EmployeeOf,
        RelationType::RelatedTo,
        RelationType::LocatedAt,
        RelationType::TransactedWith,
        RelationType::SanctionedBy,
        RelationType::RegisteredIn,
        RelationType::FlaggedAs,
        RelationType::MeetingWith,
        RelationType::TraveledTo,
        RelationType::PartOf,
    ];

    for rt in types {
        let rel = Relationship::new(src, tgt, rt.clone(), "test".to_string());
        assert_eq!(rel.relation_type, rt);
    }
}

#[test]
fn relationship_serialization_roundtrip() {
    let rel = Relationship::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        RelationType::DirectorOf,
        "opencorporates".to_string(),
    );

    let json = serde_json::to_string(&rel).expect("failed to serialize Relationship");
    let deserialized: Relationship =
        serde_json::from_str(&json).expect("failed to deserialize Relationship");

    assert_eq!(deserialized.id, rel.id);
    assert_eq!(deserialized.source_entity_id, rel.source_entity_id);
    assert_eq!(deserialized.target_entity_id, rel.target_entity_id);
    assert_eq!(deserialized.relation_type, rel.relation_type);
    assert_eq!(deserialized.source, rel.source);
    assert_eq!(deserialized.confidence, rel.confidence);
}

// ---------------------------------------------------------------------------
// ExtractionResult serialization/deserialization roundtrip
// ---------------------------------------------------------------------------

#[test]
fn extraction_result_empty_roundtrip() {
    let result = ExtractionResult {
        entities: vec![],
        relationships: vec![],
        raw_source: "empty test".to_string(),
        extracted_at: Utc::now(),
    };

    let json = serde_json::to_string(&result).expect("failed to serialize ExtractionResult");
    let deserialized: ExtractionResult =
        serde_json::from_str(&json).expect("failed to deserialize ExtractionResult");

    assert!(deserialized.entities.is_empty());
    assert!(deserialized.relationships.is_empty());
    assert_eq!(deserialized.raw_source, "empty test");
}

#[test]
fn extraction_result_with_data_roundtrip() {
    let entity1 = Entity::new(
        EntityType::Person,
        "Alice".to_string(),
        "gdelt".to_string(),
    );
    let entity2 = Entity::new(
        EntityType::Organization,
        "ACME".to_string(),
        "gdelt".to_string(),
    );

    let rel = Relationship::new(
        entity1.id,
        entity2.id,
        RelationType::EmployeeOf,
        "gdelt".to_string(),
    );

    let result = ExtractionResult {
        entities: vec![entity1.clone(), entity2.clone()],
        relationships: vec![rel.clone()],
        raw_source: "Alice works at ACME".to_string(),
        extracted_at: Utc::now(),
    };

    let json = serde_json::to_string(&result).expect("failed to serialize ExtractionResult");
    let deserialized: ExtractionResult =
        serde_json::from_str(&json).expect("failed to deserialize ExtractionResult");

    assert_eq!(deserialized.entities.len(), 2);
    assert_eq!(deserialized.relationships.len(), 1);
    assert_eq!(deserialized.raw_source, "Alice works at ACME");
    assert_eq!(deserialized.entities[0].name, "Alice");
    assert_eq!(deserialized.entities[1].name, "ACME");
    assert_eq!(
        deserialized.relationships[0].relation_type,
        RelationType::EmployeeOf
    );
    assert_eq!(
        deserialized.relationships[0].source_entity_id,
        entity1.id
    );
    assert_eq!(
        deserialized.relationships[0].target_entity_id,
        entity2.id
    );
}

#[test]
fn extraction_result_preserves_entity_ids_across_roundtrip() {
    let e = Entity::new(EntityType::Vessel, "MV Test".to_string(), "ais".to_string());
    let original_id = e.id;

    let result = ExtractionResult {
        entities: vec![e],
        relationships: vec![],
        raw_source: "vessel data".to_string(),
        extracted_at: Utc::now(),
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: ExtractionResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.entities[0].id, original_id);
}

// ---------------------------------------------------------------------------
// EntityType serialization (snake_case via serde rename_all)
// ---------------------------------------------------------------------------

#[test]
fn entity_type_serialization() {
    let cases: Vec<(EntityType, &str)> = vec![
        (EntityType::Person, "\"person\""),
        (EntityType::Organization, "\"organization\""),
        (EntityType::Vessel, "\"vessel\""),
        (EntityType::Aircraft, "\"aircraft\""),
        (EntityType::Location, "\"location\""),
        (EntityType::Event, "\"event\""),
        (EntityType::Document, "\"document\""),
        (EntityType::Transaction, "\"transaction\""),
        (EntityType::Sanction, "\"sanction\""),
    ];

    for (entity_type, expected_json) in cases {
        let json = serde_json::to_string(&entity_type)
            .expect("failed to serialize EntityType");
        assert_eq!(
            json, expected_json,
            "EntityType::{:?} serialized to '{json}' but expected '{expected_json}'",
            entity_type
        );
    }
}

#[test]
fn entity_type_deserialization() {
    let cases: Vec<(&str, EntityType)> = vec![
        ("\"person\"", EntityType::Person),
        ("\"organization\"", EntityType::Organization),
        ("\"vessel\"", EntityType::Vessel),
        ("\"aircraft\"", EntityType::Aircraft),
        ("\"location\"", EntityType::Location),
        ("\"event\"", EntityType::Event),
        ("\"document\"", EntityType::Document),
        ("\"transaction\"", EntityType::Transaction),
        ("\"sanction\"", EntityType::Sanction),
    ];

    for (json_str, expected_type) in cases {
        let deserialized: EntityType = serde_json::from_str(json_str)
            .unwrap_or_else(|e| panic!("failed to deserialize '{json_str}': {e}"));
        assert_eq!(deserialized, expected_type);
    }
}

// ---------------------------------------------------------------------------
// RelationType serialization (snake_case via serde rename_all)
// ---------------------------------------------------------------------------

#[test]
fn relation_type_serialization() {
    let cases: Vec<(RelationType, &str)> = vec![
        (RelationType::OwnerOf, "\"owner_of\""),
        (RelationType::DirectorOf, "\"director_of\""),
        (RelationType::EmployeeOf, "\"employee_of\""),
        (RelationType::RelatedTo, "\"related_to\""),
        (RelationType::LocatedAt, "\"located_at\""),
        (RelationType::TransactedWith, "\"transacted_with\""),
        (RelationType::SanctionedBy, "\"sanctioned_by\""),
        (RelationType::RegisteredIn, "\"registered_in\""),
        (RelationType::FlaggedAs, "\"flagged_as\""),
        (RelationType::MeetingWith, "\"meeting_with\""),
        (RelationType::TraveledTo, "\"traveled_to\""),
        (RelationType::PartOf, "\"part_of\""),
    ];

    for (relation_type, expected_json) in cases {
        let json = serde_json::to_string(&relation_type)
            .expect("failed to serialize RelationType");
        assert_eq!(
            json, expected_json,
            "RelationType::{:?} serialized to '{json}' but expected '{expected_json}'",
            relation_type
        );
    }
}

#[test]
fn relation_type_deserialization() {
    let cases: Vec<(&str, RelationType)> = vec![
        ("\"owner_of\"", RelationType::OwnerOf),
        ("\"director_of\"", RelationType::DirectorOf),
        ("\"employee_of\"", RelationType::EmployeeOf),
        ("\"related_to\"", RelationType::RelatedTo),
        ("\"located_at\"", RelationType::LocatedAt),
        ("\"transacted_with\"", RelationType::TransactedWith),
        ("\"sanctioned_by\"", RelationType::SanctionedBy),
        ("\"registered_in\"", RelationType::RegisteredIn),
        ("\"flagged_as\"", RelationType::FlaggedAs),
        ("\"meeting_with\"", RelationType::MeetingWith),
        ("\"traveled_to\"", RelationType::TraveledTo),
        ("\"part_of\"", RelationType::PartOf),
    ];

    for (json_str, expected_type) in cases {
        let deserialized: RelationType = serde_json::from_str(json_str)
            .unwrap_or_else(|e| panic!("failed to deserialize '{json_str}': {e}"));
        assert_eq!(deserialized, expected_type);
    }
}

// ---------------------------------------------------------------------------
// EntityType and RelationType Hash + Eq (used in HashSet/HashMap)
// ---------------------------------------------------------------------------

#[test]
fn entity_type_hash_eq() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(EntityType::Person);
    set.insert(EntityType::Person); // duplicate
    set.insert(EntityType::Organization);
    assert_eq!(set.len(), 2);
}

#[test]
fn relation_type_hash_eq() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(RelationType::OwnerOf);
    set.insert(RelationType::OwnerOf); // duplicate
    set.insert(RelationType::DirectorOf);
    assert_eq!(set.len(), 2);
}
