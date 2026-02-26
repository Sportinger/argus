"use client";

import Link from "next/link";
import type {
  Entity,
  EntityDetailResponse,
  EntityType,
  Relationship,
} from "@/types/argus";

const ENTITY_TYPE_COLORS: Record<EntityType, string> = {
  person: "bg-blue-600",
  organization: "bg-purple-600",
  vessel: "bg-cyan-600",
  aircraft: "bg-sky-600",
  location: "bg-green-600",
  event: "bg-yellow-600",
  document: "bg-gray-600",
  transaction: "bg-orange-600",
  sanction: "bg-red-600",
};

const RELATION_TYPE_COLORS: Record<string, string> = {
  owner_of: "bg-purple-500/30 text-purple-300 border-purple-500/40",
  director_of: "bg-blue-500/30 text-blue-300 border-blue-500/40",
  employee_of: "bg-sky-500/30 text-sky-300 border-sky-500/40",
  related_to: "bg-gray-500/30 text-gray-300 border-gray-500/40",
  located_at: "bg-green-500/30 text-green-300 border-green-500/40",
  transacted_with: "bg-orange-500/30 text-orange-300 border-orange-500/40",
  sanctioned_by: "bg-red-500/30 text-red-300 border-red-500/40",
  registered_in: "bg-teal-500/30 text-teal-300 border-teal-500/40",
  flagged_as: "bg-rose-500/30 text-rose-300 border-rose-500/40",
  meeting_with: "bg-indigo-500/30 text-indigo-300 border-indigo-500/40",
  traveled_to: "bg-emerald-500/30 text-emerald-300 border-emerald-500/40",
  part_of: "bg-violet-500/30 text-violet-300 border-violet-500/40",
};

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

function ConfidenceBar({ value }: { value: number }) {
  const pct = Math.round(value * 100);
  const color =
    pct >= 80 ? "bg-green-500" : pct >= 50 ? "bg-yellow-500" : "bg-red-500";
  return (
    <div className="flex items-center gap-3">
      <div className="flex-1 h-2 bg-white/10 rounded-full overflow-hidden">
        <div
          className={`h-full rounded-full ${color}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="text-sm font-mono text-neutral-400">{pct}%</span>
    </div>
  );
}

function EntityTypeBadge({
  type,
  size = "md",
}: {
  type: EntityType;
  size?: "sm" | "md";
}) {
  const sizeClasses = size === "sm" ? "px-2 py-0.5 text-xs" : "px-3 py-1 text-sm";
  return (
    <span
      className={`inline-block rounded-full font-medium text-white ${ENTITY_TYPE_COLORS[type]} ${sizeClasses}`}
    >
      {type}
    </span>
  );
}

function PropertyValue({ value }: { value: unknown }) {
  if (value === null || value === undefined) {
    return <span className="text-neutral-600 italic">null</span>;
  }
  if (typeof value === "boolean") {
    return (
      <span className={value ? "text-green-400" : "text-red-400"}>
        {String(value)}
      </span>
    );
  }
  if (typeof value === "object") {
    return (
      <code className="text-xs bg-white/5 rounded px-2 py-1 text-neutral-300 break-all">
        {JSON.stringify(value, null, 2)}
      </code>
    );
  }
  return <span className="text-neutral-200">{String(value)}</span>;
}

function resolveEntityName(
  entityId: string,
  entity: Entity,
  neighbors: Entity[]
): string {
  if (entityId === entity.id) return entity.name;
  const neighbor = neighbors.find((n) => n.id === entityId);
  return neighbor?.name ?? entityId.slice(0, 12) + "...";
}

function RelationshipCard({
  rel,
  currentEntity,
  neighbors,
}: {
  rel: Relationship;
  currentEntity: Entity;
  neighbors: Entity[];
}) {
  const isSource = rel.source_entity_id === currentEntity.id;
  const linkedEntityId = isSource
    ? rel.target_entity_id
    : rel.source_entity_id;
  const linkedName = resolveEntityName(linkedEntityId, currentEntity, neighbors);
  const linkedEntity = neighbors.find((n) => n.id === linkedEntityId);
  const relColor =
    RELATION_TYPE_COLORS[rel.relation_type] ??
    "bg-gray-500/30 text-gray-300 border-gray-500/40";

  return (
    <Link
      href={`/entity/${linkedEntityId}`}
      className="block p-4 rounded-lg bg-white/[0.03] border border-white/[0.06] hover:bg-white/[0.06] hover:border-white/[0.12] transition-all group"
    >
      <div className="flex items-center justify-between mb-2">
        <span className={`text-xs font-mono px-2 py-0.5 rounded border ${relColor}`}>
          {isSource ? "" : "<- "}
          {rel.relation_type.replace(/_/g, " ")}
          {isSource ? " ->" : ""}
        </span>
        <span className="text-xs text-neutral-500 font-mono">
          {Math.round(rel.confidence * 100)}%
        </span>
      </div>
      <div className="flex items-center gap-2">
        {linkedEntity && (
          <EntityTypeBadge type={linkedEntity.entity_type} size="sm" />
        )}
        <span className="text-neutral-200 group-hover:text-white transition-colors font-medium">
          {linkedName}
        </span>
      </div>
      {rel.timestamp && (
        <p className="text-xs text-neutral-500 mt-1">{formatDate(rel.timestamp)}</p>
      )}
    </Link>
  );
}

export default function EntityDetail({ data }: { data: EntityDetailResponse }) {
  const { entity, relationships, neighbors } = data;
  const properties = Object.entries(entity.properties);

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      {/* Header */}
      <div className="space-y-3">
        <div className="flex items-center gap-2 text-sm text-neutral-500">
          <Link href="/search" className="hover:text-neutral-300 transition-colors">
            Search
          </Link>
          <span>/</span>
          <span className="text-neutral-400">{entity.name}</span>
        </div>
        <div className="flex items-start gap-4">
          <div className="flex-1">
            <h1 className="text-3xl font-bold text-white tracking-tight">
              {entity.name}
            </h1>
            <div className="flex items-center gap-3 mt-2">
              <EntityTypeBadge type={entity.entity_type} />
              <span className="text-sm text-neutral-500 font-mono">{entity.id}</span>
            </div>
          </div>
        </div>
      </div>

      {/* Meta info cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="p-4 rounded-lg bg-white/[0.03] border border-white/[0.06]">
          <p className="text-xs text-neutral-500 uppercase tracking-wider mb-1">
            Confidence
          </p>
          <ConfidenceBar value={entity.confidence} />
        </div>
        <div className="p-4 rounded-lg bg-white/[0.03] border border-white/[0.06]">
          <p className="text-xs text-neutral-500 uppercase tracking-wider mb-1">
            Source
          </p>
          <p className="text-sm text-neutral-200 font-mono">{entity.source}</p>
        </div>
        <div className="p-4 rounded-lg bg-white/[0.03] border border-white/[0.06]">
          <p className="text-xs text-neutral-500 uppercase tracking-wider mb-1">
            First Seen
          </p>
          <p className="text-sm text-neutral-200">{formatDate(entity.first_seen)}</p>
        </div>
        <div className="p-4 rounded-lg bg-white/[0.03] border border-white/[0.06]">
          <p className="text-xs text-neutral-500 uppercase tracking-wider mb-1">
            Last Seen
          </p>
          <p className="text-sm text-neutral-200">{formatDate(entity.last_seen)}</p>
        </div>
      </div>

      {/* Aliases */}
      {entity.aliases.length > 0 && (
        <div className="space-y-3">
          <h2 className="text-lg font-semibold text-neutral-200">Aliases</h2>
          <div className="flex flex-wrap gap-2">
            {entity.aliases.map((alias) => (
              <span
                key={alias}
                className="px-3 py-1 rounded-full bg-white/[0.06] border border-white/[0.08] text-sm text-neutral-300"
              >
                {alias}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Properties */}
      {properties.length > 0 && (
        <div className="space-y-3">
          <h2 className="text-lg font-semibold text-neutral-200">Properties</h2>
          <div className="rounded-lg border border-white/[0.06] overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="bg-white/[0.03]">
                  <th className="text-left px-4 py-2 text-neutral-500 font-medium uppercase tracking-wider text-xs">
                    Key
                  </th>
                  <th className="text-left px-4 py-2 text-neutral-500 font-medium uppercase tracking-wider text-xs">
                    Value
                  </th>
                </tr>
              </thead>
              <tbody>
                {properties.map(([key, value]) => (
                  <tr
                    key={key}
                    className="border-t border-white/[0.04] hover:bg-white/[0.02]"
                  >
                    <td className="px-4 py-3 font-mono text-neutral-400">{key}</td>
                    <td className="px-4 py-3">
                      <PropertyValue value={value} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Relationships */}
      {relationships.length > 0 && (
        <div className="space-y-3">
          <h2 className="text-lg font-semibold text-neutral-200">
            Relationships{" "}
            <span className="text-neutral-500 font-normal text-base">
              ({relationships.length})
            </span>
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {relationships.map((rel) => (
              <RelationshipCard
                key={rel.id}
                rel={rel}
                currentEntity={entity}
                neighbors={neighbors}
              />
            ))}
          </div>
        </div>
      )}

      {/* Neighbors preview */}
      {neighbors.length > 0 && (
        <div className="space-y-3">
          <h2 className="text-lg font-semibold text-neutral-200">
            Connected Entities{" "}
            <span className="text-neutral-500 font-normal text-base">
              ({neighbors.length})
            </span>
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3">
            {neighbors.map((neighbor) => (
              <Link
                key={neighbor.id}
                href={`/entity/${neighbor.id}`}
                className="p-3 rounded-lg bg-white/[0.03] border border-white/[0.06] hover:bg-white/[0.06] hover:border-white/[0.12] transition-all group"
              >
                <div className="flex items-center gap-2 mb-1">
                  <EntityTypeBadge type={neighbor.entity_type} size="sm" />
                </div>
                <p className="text-sm text-neutral-200 group-hover:text-white font-medium truncate">
                  {neighbor.name}
                </p>
                <p className="text-xs text-neutral-500 mt-1 font-mono">
                  {Math.round(neighbor.confidence * 100)}% confidence
                </p>
              </Link>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
