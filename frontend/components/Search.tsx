"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { searchEntities } from "@/lib/api";
import type { Entity, EntityType } from "@/types/argus";

const ENTITY_TYPES: EntityType[] = [
  "person",
  "organization",
  "vessel",
  "aircraft",
  "location",
  "event",
  "document",
  "transaction",
  "sanction",
];

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

function EntityCard({ entity }: { entity: Entity }) {
  const pct = Math.round(entity.confidence * 100);
  const confidenceColor =
    pct >= 80 ? "text-green-400" : pct >= 50 ? "text-yellow-400" : "text-red-400";

  return (
    <Link
      href={`/entity/${entity.id}`}
      className="block p-4 rounded-lg bg-white/[0.03] border border-white/[0.06] hover:bg-white/[0.06] hover:border-white/[0.12] transition-all group"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span
              className={`inline-block px-2 py-0.5 rounded-full text-xs font-medium text-white ${ENTITY_TYPE_COLORS[entity.entity_type]}`}
            >
              {entity.entity_type}
            </span>
            <span className={`text-xs font-mono ${confidenceColor}`}>{pct}%</span>
          </div>
          <h3 className="text-base font-medium text-neutral-200 group-hover:text-white transition-colors truncate">
            {entity.name}
          </h3>
          {entity.aliases.length > 0 && (
            <p className="text-xs text-neutral-500 mt-1 truncate">
              aka: {entity.aliases.join(", ")}
            </p>
          )}
        </div>
        <div className="text-right shrink-0">
          <p className="text-xs text-neutral-500 font-mono">{entity.source}</p>
        </div>
      </div>
    </Link>
  );
}

export default function Search() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const initialQuery = searchParams.get("q") ?? "";
  const initialType = (searchParams.get("type") as EntityType | null) ?? undefined;

  const [query, setQuery] = useState(initialQuery);
  const [entityType, setEntityType] = useState<EntityType | undefined>(initialType);
  const [results, setResults] = useState<Entity[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasSearched, setHasSearched] = useState(false);

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const doSearch = useCallback(
    async (q: string, type?: EntityType) => {
      if (!q.trim()) {
        setResults([]);
        setTotal(0);
        setHasSearched(false);
        return;
      }

      // Cancel in-flight request
      if (abortRef.current) {
        abortRef.current.abort();
      }
      abortRef.current = new AbortController();

      setLoading(true);
      setError(null);
      setHasSearched(true);

      try {
        const resp = await searchEntities({
          query: q.trim(),
          limit: 50,
          entity_type: type,
        });
        setResults(resp.entities);
        setTotal(resp.total);
      } catch (err: unknown) {
        if (err instanceof DOMException && err.name === "AbortError") return;
        setError(err instanceof Error ? err.message : "Search failed");
        setResults([]);
        setTotal(0);
      } finally {
        setLoading(false);
      }
    },
    []
  );

  // Debounced search on query/type changes
  useEffect(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    debounceRef.current = setTimeout(() => {
      doSearch(query, entityType);
    }, 300);

    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, [query, entityType, doSearch]);

  // Sync URL params
  useEffect(() => {
    const params = new URLSearchParams();
    if (query.trim()) params.set("q", query.trim());
    if (entityType) params.set("type", entityType);
    const qs = params.toString();
    const newUrl = qs ? `/search?${qs}` : "/search";
    router.replace(newUrl, { scroll: false });
  }, [query, entityType, router]);

  return (
    <div className="max-w-3xl mx-auto space-y-6">
      {/* Search input */}
      <div className="space-y-3">
        <div className="relative">
          <svg
            className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-neutral-500"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
            />
          </svg>
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search entities..."
            className="w-full pl-12 pr-4 py-3 rounded-lg bg-white/[0.05] border border-white/[0.1] text-neutral-200 placeholder-neutral-500 focus:outline-none focus:border-white/[0.25] focus:bg-white/[0.07] transition-all text-base"
            autoFocus
          />
          {loading && (
            <div className="absolute right-4 top-1/2 -translate-y-1/2">
              <div className="w-4 h-4 border-2 border-neutral-500 border-t-neutral-200 rounded-full animate-spin" />
            </div>
          )}
        </div>

        {/* Entity type filter */}
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-xs text-neutral-500 uppercase tracking-wider">
            Type:
          </span>
          <button
            onClick={() => setEntityType(undefined)}
            className={`px-3 py-1 rounded-full text-xs font-medium transition-all border ${
              entityType === undefined
                ? "bg-white/[0.12] border-white/[0.2] text-white"
                : "bg-white/[0.03] border-white/[0.06] text-neutral-400 hover:bg-white/[0.06] hover:text-neutral-200"
            }`}
          >
            All
          </button>
          {ENTITY_TYPES.map((type) => (
            <button
              key={type}
              onClick={() => setEntityType(type === entityType ? undefined : type)}
              className={`px-3 py-1 rounded-full text-xs font-medium transition-all border ${
                entityType === type
                  ? "bg-white/[0.12] border-white/[0.2] text-white"
                  : "bg-white/[0.03] border-white/[0.06] text-neutral-400 hover:bg-white/[0.06] hover:text-neutral-200"
              }`}
            >
              {type}
            </button>
          ))}
        </div>
      </div>

      {/* Error state */}
      {error && (
        <div className="p-4 rounded-lg bg-red-500/10 border border-red-500/20 text-red-400 text-sm">
          {error}
        </div>
      )}

      {/* Results */}
      {hasSearched && !loading && !error && (
        <div className="space-y-3">
          <p className="text-sm text-neutral-500">
            {total} {total === 1 ? "result" : "results"} found
          </p>
          {results.length > 0 ? (
            <div className="space-y-2">
              {results.map((entity) => (
                <EntityCard key={entity.id} entity={entity} />
              ))}
            </div>
          ) : (
            <div className="text-center py-16">
              <div className="text-neutral-600 text-4xl mb-3">?</div>
              <p className="text-neutral-400 text-base">No entities found</p>
              <p className="text-neutral-600 text-sm mt-1">
                Try adjusting your search query or removing type filters
              </p>
            </div>
          )}
        </div>
      )}

      {/* Initial empty state */}
      {!hasSearched && !loading && (
        <div className="text-center py-20">
          <div className="text-neutral-700 text-5xl mb-4">
            <svg
              className="w-12 h-12 mx-auto text-neutral-700"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={1.5}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
              />
            </svg>
          </div>
          <p className="text-neutral-500 text-base">
            Search for entities by name, alias, or identifier
          </p>
          <p className="text-neutral-600 text-sm mt-1">
            Filter by type to narrow results
          </p>
        </div>
      )}
    </div>
  );
}
