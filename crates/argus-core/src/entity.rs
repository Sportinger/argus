use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Organization,
    Vessel,
    Aircraft,
    Location,
    Event,
    Document,
    Transaction,
    Sanction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub entity_type: EntityType,
    pub name: String,
    pub aliases: Vec<String>,
    pub properties: serde_json::Value,
    pub source: String,
    pub source_id: Option<String>,
    pub confidence: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

impl Entity {
    pub fn new(entity_type: EntityType, name: String, source: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            entity_type,
            name,
            aliases: Vec::new(),
            properties: serde_json::Value::Object(serde_json::Map::new()),
            source,
            source_id: None,
            confidence: 1.0,
            first_seen: now,
            last_seen: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    OwnerOf,
    DirectorOf,
    EmployeeOf,
    RelatedTo,
    LocatedAt,
    TransactedWith,
    SanctionedBy,
    RegisteredIn,
    FlaggedAs,
    MeetingWith,
    TraveledTo,
    PartOf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: Uuid,
    pub source_entity_id: Uuid,
    pub target_entity_id: Uuid,
    pub relation_type: RelationType,
    pub properties: serde_json::Value,
    pub confidence: f64,
    pub source: String,
    pub timestamp: Option<DateTime<Utc>>,
}

impl Relationship {
    pub fn new(
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: RelationType,
        source: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_entity_id,
            target_entity_id,
            relation_type,
            properties: serde_json::Value::Object(serde_json::Map::new()),
            confidence: 1.0,
            source,
            timestamp: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
    pub raw_source: String,
    pub extracted_at: DateTime<Utc>,
}
