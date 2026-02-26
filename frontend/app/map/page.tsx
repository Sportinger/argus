"use client";

import { useEffect, useState } from "react";
import MapView from "@/components/MapView";
import { searchEntities } from "@/lib/api";
import type { Entity } from "@/types/argus";

export default function MapPage() {
  const [entities, setEntities] = useState<Entity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function fetchEntities() {
      try {
        setLoading(true);
        const response = await searchEntities({ query: "*", limit: 1000 });
        const withLocation = response.entities.filter(
          (e) => {
            const props = e.properties as Record<string, unknown>;
            return props?.latitude != null && props?.longitude != null;
          }
        );
        setEntities(withLocation);
      } catch (err) {
        setError(
          err instanceof Error ? err.message : "Failed to fetch entities"
        );
      } finally {
        setLoading(false);
      }
    }

    fetchEntities();
  }, []);

  return (
    <div className="flex flex-col h-screen bg-zinc-950">
      <nav className="flex items-center justify-between border-b border-zinc-800 px-6 py-3">
        <a href="/" className="text-lg font-bold tracking-tight text-zinc-100">
          ARGUS
        </a>
        <div className="flex items-center gap-4">
          <span className="text-sm text-zinc-400">
            {loading
              ? "Loading..."
              : `${entities.length} entities on map`}
          </span>
          <a
            href="/map"
            className="text-sm font-medium text-zinc-100 underline underline-offset-4"
          >
            Map
          </a>
        </div>
      </nav>

      <main className="relative flex-1">
        {error && (
          <div className="absolute inset-x-0 top-4 z-20 mx-auto max-w-md rounded-md border border-red-800 bg-red-950/90 px-4 py-3 text-center text-sm text-red-300">
            {error}
          </div>
        )}

        {loading ? (
          <div className="flex h-full items-center justify-center">
            <div className="text-zinc-500 text-sm">Loading map data...</div>
          </div>
        ) : (
          <MapView entities={entities} />
        )}
      </main>
    </div>
  );
}
