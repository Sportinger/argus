"use client";

import { useEffect, useState } from "react";
import { getHealth, listAgents } from "@/lib/api";
import type { HealthResponse, AgentStatus } from "@/types/argus";

interface DashboardState {
  health: HealthResponse | null;
  agents: AgentStatus[];
  loading: boolean;
  error: string | null;
}

function StatusDot({ connected }: { connected: boolean }) {
  return (
    <span
      className={`inline-block h-2.5 w-2.5 rounded-full ${
        connected ? "bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.6)]" : "bg-red-500 shadow-[0_0_6px_rgba(239,68,68,0.6)]"
      }`}
    />
  );
}

function StatCard({
  title,
  value,
  subtitle,
}: {
  title: string;
  value: string | number;
  subtitle?: string;
}) {
  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900/60 p-5">
      <p className="text-xs font-medium uppercase tracking-wider text-zinc-500">
        {title}
      </p>
      <p className="mt-2 text-3xl font-bold text-zinc-100">{value}</p>
      {subtitle && (
        <p className="mt-1 text-sm text-zinc-500">{subtitle}</p>
      )}
    </div>
  );
}

function ConnectionCard({
  name,
  connected,
}: {
  name: string;
  connected: boolean;
}) {
  return (
    <div className="flex items-center justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-5">
      <div>
        <p className="text-sm font-medium text-zinc-300">{name}</p>
        <p className="text-xs text-zinc-500 mt-1">
          {connected ? "Connected" : "Disconnected"}
        </p>
      </div>
      <StatusDot connected={connected} />
    </div>
  );
}

function AgentCard({ agent }: { agent: AgentStatus }) {
  return (
    <div className="flex items-center justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4">
      <div className="flex items-center gap-3">
        <StatusDot connected={agent.enabled && !agent.error} />
        <div>
          <p className="text-sm font-medium text-zinc-200">{agent.name}</p>
          {agent.last_run && (
            <p className="text-xs text-zinc-500 mt-0.5">
              Last run: {new Date(agent.last_run).toLocaleString()}
            </p>
          )}
          {agent.error && (
            <p className="text-xs text-red-400 mt-0.5">{agent.error}</p>
          )}
        </div>
      </div>
      <div className="text-right">
        <p className="text-sm font-mono text-zinc-400">
          {agent.documents_collected}
        </p>
        <p className="text-xs text-zinc-600">docs</p>
      </div>
    </div>
  );
}

export default function Dashboard() {
  const [state, setState] = useState<DashboardState>({
    health: null,
    agents: [],
    loading: true,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;

    async function fetchData() {
      try {
        const [health, agentList] = await Promise.all([
          getHealth(),
          listAgents(),
        ]);
        if (!cancelled) {
          setState({
            health,
            agents: agentList.agents,
            loading: false,
            error: null,
          });
        }
      } catch (err) {
        if (!cancelled) {
          setState((prev) => ({
            ...prev,
            loading: false,
            error: err instanceof Error ? err.message : "Failed to fetch data",
          }));
        }
      }
    }

    fetchData();
    return () => {
      cancelled = true;
    };
  }, []);

  if (state.loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="flex flex-col items-center gap-3">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-zinc-700 border-t-emerald-500" />
          <p className="text-sm text-zinc-500">Loading dashboard...</p>
        </div>
      </div>
    );
  }

  if (state.error) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="rounded-xl border border-red-900/50 bg-red-950/30 p-6 max-w-md text-center">
          <p className="text-sm font-medium text-red-400">
            Failed to connect to ARGUS API
          </p>
          <p className="mt-2 text-xs text-zinc-500">{state.error}</p>
        </div>
      </div>
    );
  }

  const { health, agents } = state;

  return (
    <div className="p-6 lg:p-8 space-y-8 max-w-7xl mx-auto">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-zinc-100">Dashboard</h1>
        <p className="mt-1 text-sm text-zinc-500">
          ARGUS system overview and status
        </p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          title="Entities"
          value={health?.entity_count.toLocaleString() ?? "0"}
          subtitle="Total entities tracked"
        />
        <StatCard
          title="Relationships"
          value={health?.relationship_count.toLocaleString() ?? "0"}
          subtitle="Connections mapped"
        />
        <StatCard
          title="Agents"
          value={agents.length}
          subtitle={`${agents.filter((a) => a.enabled).length} active`}
        />
        <StatCard
          title="Status"
          value={health?.status === "ok" ? "Operational" : "Degraded"}
          subtitle={`v${health?.version ?? "?"}`}
        />
      </div>

      {/* Connections */}
      <div>
        <h2 className="text-lg font-semibold text-zinc-200 mb-3">
          Infrastructure
        </h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <ConnectionCard
            name="Neo4j Graph Database"
            connected={health?.neo4j_connected ?? false}
          />
          <ConnectionCard
            name="Qdrant Vector Store"
            connected={health?.qdrant_connected ?? false}
          />
        </div>
      </div>

      {/* Agents */}
      {agents.length > 0 && (
        <div>
          <h2 className="text-lg font-semibold text-zinc-200 mb-3">
            Ingestion Agents
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {agents.map((agent) => (
              <AgentCard key={agent.name} agent={agent} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
