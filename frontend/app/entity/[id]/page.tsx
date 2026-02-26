"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { getEntity } from "@/lib/api";
import type { EntityDetailResponse } from "@/types/argus";
import EntityDetail from "@/components/EntityDetail";

export default function EntityPage() {
  const params = useParams<{ id: string }>();
  const entityId = params.id;

  const [data, setData] = useState<EntityDetailResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!entityId) return;

    let cancelled = false;
    setLoading(true);
    setError(null);
    setData(null);

    getEntity(entityId)
      .then((resp) => {
        if (!cancelled) {
          setData(resp);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load entity");
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [entityId]);

  if (loading) {
    return (
      <div className="min-h-screen bg-[#0a0a0a] p-8">
        <div className="max-w-4xl mx-auto">
          <div className="animate-pulse space-y-6">
            {/* Breadcrumb skeleton */}
            <div className="h-4 w-32 bg-white/[0.06] rounded" />
            {/* Title skeleton */}
            <div className="space-y-3">
              <div className="h-8 w-80 bg-white/[0.06] rounded" />
              <div className="flex gap-3">
                <div className="h-6 w-20 bg-white/[0.06] rounded-full" />
                <div className="h-6 w-48 bg-white/[0.06] rounded" />
              </div>
            </div>
            {/* Meta cards skeleton */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              {Array.from({ length: 4 }).map((_, i) => (
                <div
                  key={i}
                  className="h-20 bg-white/[0.03] border border-white/[0.06] rounded-lg"
                />
              ))}
            </div>
            {/* Properties skeleton */}
            <div className="space-y-2">
              <div className="h-6 w-24 bg-white/[0.06] rounded" />
              {Array.from({ length: 3 }).map((_, i) => (
                <div key={i} className="h-10 bg-white/[0.03] rounded" />
              ))}
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-[#0a0a0a] p-8">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-20">
            <div className="text-red-500 text-5xl mb-4">!</div>
            <h2 className="text-xl font-semibold text-neutral-200 mb-2">
              Failed to load entity
            </h2>
            <p className="text-neutral-500 mb-6">{error}</p>
            <div className="flex items-center justify-center gap-4">
              <button
                onClick={() => window.location.reload()}
                className="px-4 py-2 rounded-lg bg-white/[0.06] border border-white/[0.1] text-neutral-300 hover:bg-white/[0.1] transition-all text-sm"
              >
                Retry
              </button>
              <Link
                href="/search"
                className="px-4 py-2 rounded-lg bg-white/[0.06] border border-white/[0.1] text-neutral-300 hover:bg-white/[0.1] transition-all text-sm"
              >
                Back to Search
              </Link>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (!data) return null;

  return (
    <div className="min-h-screen bg-[#0a0a0a] p-8">
      <EntityDetail data={data} />
    </div>
  );
}
