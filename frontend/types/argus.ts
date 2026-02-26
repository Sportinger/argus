// ARGUS TypeScript API Types — mirrors Rust API types

export type EntityType =
  | "person"
  | "organization"
  | "vessel"
  | "aircraft"
  | "location"
  | "event"
  | "document"
  | "transaction"
  | "sanction";

export type RelationType =
  | "owner_of"
  | "director_of"
  | "employee_of"
  | "related_to"
  | "located_at"
  | "transacted_with"
  | "sanctioned_by"
  | "registered_in"
  | "flagged_as"
  | "meeting_with"
  | "traveled_to"
  | "part_of";

export interface Entity {
  id: string;
  entity_type: EntityType;
  name: string;
  aliases: string[];
  properties: Record<string, unknown>;
  source: string;
  source_id: string | null;
  confidence: number;
  first_seen: string;
  last_seen: string;
}

export interface Relationship {
  id: string;
  source_entity_id: string;
  target_entity_id: string;
  relation_type: RelationType;
  properties: Record<string, unknown>;
  confidence: number;
  source: string;
  timestamp: string | null;
}

// --- Health ---

export interface HealthResponse {
  status: string;
  version: string;
  neo4j_connected: boolean;
  qdrant_connected: boolean;
  entity_count: number;
  relationship_count: number;
}

// --- Agents ---

export interface AgentStatus {
  name: string;
  enabled: boolean;
  last_run: string | null;
  documents_collected: number;
  error: string | null;
}

export interface AgentListResponse {
  agents: AgentStatus[];
}

export interface AgentTriggerRequest {
  agent_name: string;
}

export interface AgentTriggerResponse {
  agent_name: string;
  documents_collected: number;
  message: string;
}

// --- Entities ---

export interface EntitySearchRequest {
  query: string;
  limit?: number;
  entity_type?: EntityType;
}

export interface EntitySearchResponse {
  entities: Entity[];
  total: number;
}

export interface EntityDetailResponse {
  entity: Entity;
  relationships: Relationship[];
  neighbors: Entity[];
}

// --- Graph ---

export interface GraphQueryRequest {
  cypher: string;
  params?: Record<string, unknown>;
}

export interface GraphQueryResponse {
  result: unknown;
}

export interface EntityTypeStat {
  entity_type: EntityType;
  count: number;
}

export interface GraphStatsResponse {
  entity_count: number;
  relationship_count: number;
  entity_types: EntityTypeStat[];
}

// --- Reasoning ---

export interface ReasoningRequest {
  question: string;
  context?: string;
  max_hops?: number;
}

export interface ReasoningStep {
  description: string;
  cypher: string | null;
  result_summary: string;
}

export interface ReasoningResponse {
  answer: string;
  confidence: number;
  steps: ReasoningStep[];
  entities_referenced: Entity[];
  sources: string[];
}

// --- Timeline ---

export interface TimelineRequest {
  entity_id?: string;
  start?: string;
  end?: string;
  limit?: number;
}

export interface TimelineEvent {
  timestamp: string;
  entity: Entity;
  event_type: string;
  description: string;
  source: string;
}

export interface TimelineResponse {
  events: TimelineEvent[];
}

// --- Graph Viz ---

export interface GraphNode {
  id: string;
  label: string;
  entity_type: EntityType;
  properties: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  relation_type: RelationType;
  properties: Record<string, unknown>;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}
