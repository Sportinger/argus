import type {
  AgentListResponse,
  AgentTriggerRequest,
  AgentTriggerResponse,
  EntityDetailResponse,
  EntitySearchRequest,
  EntitySearchResponse,
  GraphQueryRequest,
  GraphQueryResponse,
  GraphStatsResponse,
  HealthResponse,
  ReasoningRequest,
  ReasoningResponse,
  TimelineRequest,
  TimelineResponse,
} from "@/types/argus";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

async function fetchApi<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`API error ${res.status}: ${text}`);
  }
  return res.json();
}

// Health
export function getHealth(): Promise<HealthResponse> {
  return fetchApi("/api/health");
}

// Agents
export function listAgents(): Promise<AgentListResponse> {
  return fetchApi("/api/agents");
}

export function triggerAgent(req: AgentTriggerRequest): Promise<AgentTriggerResponse> {
  return fetchApi("/api/agents/trigger", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

// Entities
export function searchEntities(req: EntitySearchRequest): Promise<EntitySearchResponse> {
  return fetchApi("/api/entities/search", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export function getEntity(id: string): Promise<EntityDetailResponse> {
  return fetchApi(`/api/entities/${id}`);
}

// Graph
export function queryGraph(req: GraphQueryRequest): Promise<GraphQueryResponse> {
  return fetchApi("/api/graph/query", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export function getGraphStats(): Promise<GraphStatsResponse> {
  return fetchApi("/api/graph/stats");
}

export function getNeighbors(entityId: string): Promise<EntityDetailResponse> {
  return fetchApi(`/api/graph/neighbors/${entityId}`);
}

// Reasoning
export function queryReasoning(req: ReasoningRequest): Promise<ReasoningResponse> {
  return fetchApi("/api/reasoning/query", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

// Timeline
export function getTimeline(req: TimelineRequest): Promise<TimelineResponse> {
  return fetchApi("/api/timeline", {
    method: "POST",
    body: JSON.stringify(req),
  });
}
