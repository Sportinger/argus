"use client";

import { useState, useCallback } from "react";
import type { TimelineEvent } from "@/types/argus";
import { getTimeline } from "@/lib/api";
import Timeline from "@/components/Timeline";

function todayISO(): string {
  return new Date().toISOString().slice(0, 10);
}

function thirtyDaysAgoISO(): string {
  const d = new Date();
  d.setDate(d.getDate() - 30);
  return d.toISOString().slice(0, 10);
}

export default function TimelinePage() {
  const [startDate, setStartDate] = useState(thirtyDaysAgoISO);
  const [endDate, setEndDate] = useState(todayISO);
  const [entityFilter, setEntityFilter] = useState("");
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasFetched, setHasFetched] = useState(false);

  const fetchTimeline = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const resp = await getTimeline({
        start: startDate ? `${startDate}T00:00:00Z` : undefined,
        end: endDate ? `${endDate}T23:59:59Z` : undefined,
        entity_id: entityFilter.trim() || undefined,
        limit: 200,
      });
      setEvents(resp.events);
      setHasFetched(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch timeline");
      setEvents([]);
    } finally {
      setLoading(false);
    }
  }, [startDate, endDate, entityFilter]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    fetchTimeline();
  };

  return (
    <div className="min-h-screen bg-zinc-900 text-zinc-100">
      {/* Header */}
      <header className="border-b border-zinc-800 bg-zinc-900/80 backdrop-blur-sm">
        <div className="mx-auto flex max-w-6xl items-center gap-4 px-6 py-4">
          <a href="/" className="text-lg font-bold tracking-tight text-zinc-100">
            ARGUS
          </a>
          <span className="text-zinc-600">/</span>
          <h1 className="text-lg font-medium text-zinc-300">Timeline</h1>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-6 py-8">
        {/* Filter form */}
        <form
          onSubmit={handleSubmit}
          className="mb-10 rounded-xl border border-zinc-800 bg-zinc-900/50 p-6"
        >
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {/* Start date */}
            <div className="flex flex-col gap-1.5">
              <label
                htmlFor="start-date"
                className="text-xs font-medium uppercase tracking-wider text-zinc-500"
              >
                Start Date
              </label>
              <input
                id="start-date"
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
                className="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 outline-none transition focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
              />
            </div>

            {/* End date */}
            <div className="flex flex-col gap-1.5">
              <label
                htmlFor="end-date"
                className="text-xs font-medium uppercase tracking-wider text-zinc-500"
              >
                End Date
              </label>
              <input
                id="end-date"
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
                className="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 outline-none transition focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
              />
            </div>

            {/* Entity filter */}
            <div className="flex flex-col gap-1.5">
              <label
                htmlFor="entity-filter"
                className="text-xs font-medium uppercase tracking-wider text-zinc-500"
              >
                Entity ID (optional)
              </label>
              <input
                id="entity-filter"
                type="text"
                value={entityFilter}
                onChange={(e) => setEntityFilter(e.target.value)}
                placeholder="e.g. entity-uuid"
                className="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 outline-none transition focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
              />
            </div>

            {/* Submit */}
            <div className="flex flex-col justify-end">
              <button
                type="submit"
                disabled={loading}
                className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {loading ? "Loading..." : "Fetch Timeline"}
              </button>
            </div>
          </div>
        </form>

        {/* Error state */}
        {error && (
          <div className="mb-8 rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-400">
            {error}
          </div>
        )}

        {/* Loading state */}
        {loading && (
          <div className="flex flex-col items-center justify-center py-20">
            <div className="h-8 w-8 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-500" />
            <p className="mt-4 text-sm text-zinc-500">Fetching timeline events...</p>
          </div>
        )}

        {/* Results */}
        {!loading && hasFetched && (
          <>
            <div className="mb-6 flex items-center justify-between">
              <p className="text-sm text-zinc-500">
                {events.length} event{events.length !== 1 ? "s" : ""} found
              </p>
            </div>
            <Timeline events={events} />
          </>
        )}

        {/* Initial state before any fetch */}
        {!loading && !hasFetched && !error && (
          <div className="flex flex-col items-center justify-center py-20 text-zinc-500">
            <svg
              className="mb-4 h-12 w-12"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={1.5}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"
              />
            </svg>
            <p className="text-lg font-medium">Event Timeline</p>
            <p className="mt-1 text-sm">
              Select a date range and click Fetch Timeline to view events.
            </p>
          </div>
        )}
      </main>
    </div>
  );
}
