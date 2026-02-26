"use client";

import { useRef, useEffect, useCallback } from "react";
import * as d3 from "d3";
import type { GraphData, GraphNode, GraphEdge, EntityType } from "@/types/argus";

const ENTITY_COLORS: Record<EntityType, string> = {
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

const NODE_RADIUS = 20;

interface SimNode extends d3.SimulationNodeDatum {
  id: string;
  label: string;
  entity_type: EntityType;
  properties: Record<string, unknown>;
}

interface SimLink extends d3.SimulationLinkDatum<SimNode> {
  id: string;
  relation_type: string;
  properties: Record<string, unknown>;
}

interface GraphVizProps {
  data: GraphData;
  onNodeClick?: (nodeId: string) => void;
}

export default function GraphViz({ data, onNodeClick }: GraphVizProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const simulationRef = useRef<d3.Simulation<SimNode, SimLink> | null>(null);

  const buildSimData = useCallback(
    (graphData: GraphData): { nodes: SimNode[]; links: SimLink[] } => {
      const nodeMap = new Set(graphData.nodes.map((n) => n.id));

      const nodes: SimNode[] = graphData.nodes.map((n) => ({
        id: n.id,
        label: n.label,
        entity_type: n.entity_type,
        properties: n.properties,
      }));

      const links: SimLink[] = graphData.edges
        .filter((e) => nodeMap.has(e.source) && nodeMap.has(e.target))
        .map((e) => ({
          id: e.id,
          source: e.source,
          target: e.target,
          relation_type: e.relation_type,
          properties: e.properties,
        }));

      return { nodes, links };
    },
    []
  );

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;

    const width = svg.clientWidth || 900;
    const height = svg.clientHeight || 600;

    // Clear previous content
    d3.select(svg).selectAll("*").remove();

    if (data.nodes.length === 0) return;

    const { nodes, links } = buildSimData(data);

    const svgSel = d3
      .select(svg)
      .attr("viewBox", `0 0 ${width} ${height}`)
      .attr("preserveAspectRatio", "xMidYMid meet");

    // Container for zoom/pan
    const container = svgSel.append("g").attr("class", "graph-container");

    // Zoom behavior
    const zoom = d3
      .zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 5])
      .on("zoom", (event) => {
        container.attr("transform", event.transform);
      });

    svgSel.call(zoom);

    // Arrow marker for directed edges
    svgSel
      .append("defs")
      .append("marker")
      .attr("id", "arrowhead")
      .attr("viewBox", "0 -5 10 10")
      .attr("refX", NODE_RADIUS + 10)
      .attr("refY", 0)
      .attr("markerWidth", 6)
      .attr("markerHeight", 6)
      .attr("orient", "auto")
      .append("path")
      .attr("d", "M0,-5L10,0L0,5")
      .attr("fill", "#6b7280");

    // Force simulation
    const simulation = d3
      .forceSimulation<SimNode>(nodes)
      .force(
        "link",
        d3
          .forceLink<SimNode, SimLink>(links)
          .id((d) => d.id)
          .distance(150)
      )
      .force("charge", d3.forceManyBody().strength(-400))
      .force("center", d3.forceCenter(width / 2, height / 2))
      .force("collision", d3.forceCollide().radius(NODE_RADIUS + 10));

    simulationRef.current = simulation;

    // Draw edges
    const linkGroup = container
      .append("g")
      .attr("class", "links")
      .selectAll("g")
      .data(links)
      .join("g");

    const linkLines = linkGroup
      .append("line")
      .attr("stroke", "#4b5563")
      .attr("stroke-width", 1.5)
      .attr("stroke-opacity", 0.7)
      .attr("marker-end", "url(#arrowhead)");

    const linkLabels = linkGroup
      .append("text")
      .text((d) => d.relation_type.replace(/_/g, " "))
      .attr("fill", "#9ca3af")
      .attr("font-size", "9px")
      .attr("text-anchor", "middle")
      .attr("dy", -6);

    // Draw nodes
    const nodeGroup = container
      .append("g")
      .attr("class", "nodes")
      .selectAll("g")
      .data(nodes)
      .join("g")
      .attr("cursor", "pointer");

    // Drag behavior
    const drag = d3
      .drag<SVGGElement, SimNode>()
      .on("start", (event, d) => {
        if (!event.active) simulation.alphaTarget(0.3).restart();
        d.fx = d.x;
        d.fy = d.y;
      })
      .on("drag", (event, d) => {
        d.fx = event.x;
        d.fy = event.y;
      })
      .on("end", (event, d) => {
        if (!event.active) simulation.alphaTarget(0);
        d.fx = null;
        d.fy = null;
      });

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    nodeGroup.call(drag as any);

    // Node circles
    nodeGroup
      .append("circle")
      .attr("r", NODE_RADIUS)
      .attr("fill", (d) => ENTITY_COLORS[d.entity_type] || "#8b5cf6")
      .attr("stroke", "#1f2937")
      .attr("stroke-width", 2)
      .attr("opacity", 0.9);

    // Node type icon (first letter as shorthand)
    nodeGroup
      .append("text")
      .text((d) => d.entity_type.charAt(0).toUpperCase())
      .attr("fill", "#111827")
      .attr("font-size", "14px")
      .attr("font-weight", "bold")
      .attr("text-anchor", "middle")
      .attr("dy", "0.35em");

    // Node labels (name below node)
    nodeGroup
      .append("text")
      .text((d) => {
        const maxLen = 18;
        return d.label.length > maxLen
          ? d.label.slice(0, maxLen) + "..."
          : d.label;
      })
      .attr("fill", "#e5e7eb")
      .attr("font-size", "11px")
      .attr("text-anchor", "middle")
      .attr("dy", NODE_RADIUS + 14);

    // Click handler for expanding neighbors
    nodeGroup.on("click", (_event, d) => {
      if (onNodeClick) {
        onNodeClick(d.id);
      }
    });

    // Hover effects
    nodeGroup
      .on("mouseenter", function () {
        d3.select(this)
          .select("circle")
          .transition()
          .duration(150)
          .attr("r", NODE_RADIUS + 4)
          .attr("stroke", "#e5e7eb")
          .attr("stroke-width", 3);
      })
      .on("mouseleave", function () {
        d3.select(this)
          .select("circle")
          .transition()
          .duration(150)
          .attr("r", NODE_RADIUS)
          .attr("stroke", "#1f2937")
          .attr("stroke-width", 2);
      });

    // Simulation tick
    simulation.on("tick", () => {
      linkLines
        .attr("x1", (d) => (d.source as SimNode).x ?? 0)
        .attr("y1", (d) => (d.source as SimNode).y ?? 0)
        .attr("x2", (d) => (d.target as SimNode).x ?? 0)
        .attr("y2", (d) => (d.target as SimNode).y ?? 0);

      linkLabels
        .attr("x", (d) => {
          const sx = (d.source as SimNode).x ?? 0;
          const tx = (d.target as SimNode).x ?? 0;
          return (sx + tx) / 2;
        })
        .attr("y", (d) => {
          const sy = (d.source as SimNode).y ?? 0;
          const ty = (d.target as SimNode).y ?? 0;
          return (sy + ty) / 2;
        });

      nodeGroup.attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
    });

    return () => {
      simulation.stop();
    };
  }, [data, buildSimData, onNodeClick]);

  return (
    <svg
      ref={svgRef}
      className="w-full h-full"
      style={{
        background: "#0f172a",
        borderRadius: "8px",
        minHeight: "600px",
      }}
    />
  );
}
