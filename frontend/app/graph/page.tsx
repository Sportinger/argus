"use client";

import { useState, useCallback } from "react";
import GraphViz from "@/components/GraphViz";
import { searchEntities, getNeighbors } from "@/lib/api";
import type { GraphData, GraphNode, GraphEdge, EntityType } from "@/types/argus";
import type { EntityDetailResponse, Entity, Relationship } from "@/types/argus";

function entityToNode(entity: Entity): GraphNode {
  return {
    id: entity.id,
    label: entity.name,
    entity_type: entity.entity_type,
    properties: entity.properties,
  };
}

function relationshipToEdge(rel: Relationship): GraphEdge {
  return {
    id: rel.id,
    source: rel.source_entity_id,
    target: rel.target_entity_id,
    relation_type: rel.relation_type,
    properties: rel.properties,
  };
}

function mergeGraphData(existing: GraphData, detail: EntityDetailResponse): GraphData {
  const nodeMap = new Map<string, GraphNode>();
  const edgeMap = new Map<string, GraphEdge>();

  for (const node of existing.nodes) {
    nodeMap.set(node.id, node);
  }
  for (const edge of existing.edges) {
    edgeMap.set(edge.id, edge);
  }

  // Add the primary entity
  nodeMap.set(detail.entity.id, entityToNode(detail.entity));

  // Add neighbor entities
  for (const neighbor of detail.neighbors) {
    nodeMap.set(neighbor.id, entityToNode(neighbor));
  }

  // Add relationships
  for (const rel of detail.relationships) {
    edgeMap.set(rel.id, relationshipToEdge(rel));
  }

  return {
    nodes: Array.from(nodeMap.values()),
    edges: Array.from(edgeMap.values()),
  };
}

const ENTITY_TYPE_COLORS: Record<EntityType, string> = {
  person: "#4a90d9",
  organization: "#50c878",
  vessel: "#00ced1",
  aircraft: "#f0e068",
  location: "#e74c3c",
  event: "#f39c12",
  document: "#a78bfa",
  transaction: "#f472b6",
  sanction: "#ef4444",
};

export default function GraphPage() {
  const [query, setQuery] = useState("");
  const [graphData, setGraphData] = useState<GraphData>({ nodes: [], edges: [] });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  const handleSearch = useCallback(async () => {
    if (!query.trim()) return;

    setLoading(true);
    setError(null);

    try {
      const searchResult = await searchEntities({ query: query.trim(), limit: 5 });

      if (searchResult.entities.length === 0) {
        setError("No entities found for that query.");
        setLoading(false);
        return;
      }

      // Use the first matching entity as the starting point
      const startEntity = searchResult.entities[0];
      const detail = await getNeighbors(startEntity.id);

      const newGraph = mergeGraphData({ nodes: [], edges: [] }, detail);
      setGraphData(newGraph);
      setSelectedNode(startEntity.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch graph data.");
    } finally {
      setLoading(false);
    }
  }, [query]);

  const handleNodeClick = useCallback(
    async (nodeId: string) => {
      setLoading(true);
      setError(null);
      setSelectedNode(nodeId);

      try {
        const detail = await getNeighbors(nodeId);
        setGraphData((prev) => mergeGraphData(prev, detail));
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to expand node.");
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSearch();
    }
  };

  const handleClear = () => {
    setGraphData({ nodes: [], edges: [] });
    setSelectedNode(null);
    setError(null);
  };

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100 flex flex-col">
      {/* Header */}
      <header className="border-b border-gray-800 px-6 py-4">
        <div className="max-w-screen-2xl mx-auto flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-white tracking-tight">
              ARGUS Graph Explorer
            </h1>
            <p className="text-sm text-gray-400 mt-1">
              Interactive knowledge graph visualization
            </p>
          </div>
          <div className="flex items-center gap-3 text-sm text-gray-400">
            <span>{graphData.nodes.length} nodes</span>
            <span className="text-gray-600">|</span>
            <span>{graphData.edges.length} edges</span>
          </div>
        </div>
      </header>

      {/* Controls */}
      <div className="border-b border-gray-800 px-6 py-3">
        <div className="max-w-screen-2xl mx-auto flex items-center gap-3">
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search for an entity to start exploring..."
            className="flex-1 bg-gray-900 border border-gray-700 rounded-lg px-4 py-2 text-gray-100 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          <button
            onClick={handleSearch}
            disabled={loading || !query.trim()}
            className="bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:text-gray-500 text-white font-medium px-5 py-2 rounded-lg transition-colors"
          >
            {loading ? "Loading..." : "Search"}
          </button>
          {graphData.nodes.length > 0 && (
            <button
              onClick={handleClear}
              className="bg-gray-800 hover:bg-gray-700 text-gray-300 font-medium px-4 py-2 rounded-lg transition-colors"
            >
              Clear
            </button>
          )}
        </div>

        {/* Legend */}
        <div className="max-w-screen-2xl mx-auto mt-3 flex flex-wrap gap-4 text-xs">
          {Object.entries(ENTITY_TYPE_COLORS).map(([type, color]) => (
            <div key={type} className="flex items-center gap-1.5">
              <span
                className="w-3 h-3 rounded-full inline-block"
                style={{ backgroundColor: color }}
              />
              <span className="text-gray-400 capitalize">{type}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Error banner */}
      {error && (
        <div className="px-6 py-3 bg-red-900/30 border-b border-red-800">
          <div className="max-w-screen-2xl mx-auto text-red-400 text-sm">
            {error}
          </div>
        </div>
      )}

      {/* Graph area */}
      <div className="flex-1 relative">
        {graphData.nodes.length > 0 ? (
          <div className="absolute inset-0 p-4">
            <GraphViz data={graphData} onNodeClick={handleNodeClick} />
          </div>
        ) : (
          <div className="flex items-center justify-center h-full min-h-[500px] text-gray-500">
            <div className="text-center">
              <svg
                className="w-16 h-16 mx-auto mb-4 text-gray-700"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
                />
              </svg>
              <p className="text-lg">Search for an entity to explore the knowledge graph</p>
              <p className="text-sm text-gray-600 mt-2">
                Click on nodes to expand their connections
              </p>
            </div>
          </div>
        )}

        {/* Loading overlay */}
        {loading && graphData.nodes.length > 0 && (
          <div className="absolute top-6 right-6 bg-gray-900/90 border border-gray-700 rounded-lg px-4 py-2 text-sm text-gray-300">
            Expanding graph...
          </div>
        )}

        {/* Selected node info */}
        {selectedNode && graphData.nodes.length > 0 && (
          <div className="absolute bottom-6 left-6 bg-gray-900/95 border border-gray-700 rounded-lg px-4 py-3 text-sm max-w-xs">
            {(() => {
              const node = graphData.nodes.find((n) => n.id === selectedNode);
              if (!node) return null;
              return (
                <div>
                  <div className="flex items-center gap-2 mb-1">
                    <span
                      className="w-3 h-3 rounded-full inline-block"
                      style={{
                        backgroundColor:
                          ENTITY_TYPE_COLORS[node.entity_type] || "#8b5cf6",
                      }}
                    />
                    <span className="font-medium text-white">{node.label}</span>
                  </div>
                  <span className="text-gray-400 capitalize text-xs">
                    {node.entity_type}
                  </span>
                </div>
              );
            })()}
          </div>
        )}
      </div>
    </div>
  );
}
