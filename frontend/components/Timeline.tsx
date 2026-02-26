"use client";

import type { TimelineEvent, EntityType } from "@/types/argus";

interface TimelineProps {
  events: TimelineEvent[];
}

const ENTITY_COLORS: Record<EntityType, string> = {
  person: "#3b82f6",       // blue-500
  organization: "#8b5cf6", // violet-500
  vessel: "#06b6d4",       // cyan-500
  aircraft: "#f59e0b",     // amber-500
  location: "#22c55e",     // green-500
  event: "#ef4444",        // red-500
  document: "#6b7280",     // gray-500
  transaction: "#f97316",  // orange-500
  sanction: "#dc2626",     // red-600
};

const ENTITY_BG_COLORS: Record<EntityType, string> = {
  person: "bg-blue-500/15 border-blue-500/30",
  organization: "bg-violet-500/15 border-violet-500/30",
  vessel: "bg-cyan-500/15 border-cyan-500/30",
  aircraft: "bg-amber-500/15 border-amber-500/30",
  location: "bg-green-500/15 border-green-500/30",
  event: "bg-red-500/15 border-red-500/30",
  document: "bg-gray-500/15 border-gray-500/30",
  transaction: "bg-orange-500/15 border-orange-500/30",
  sanction: "bg-red-600/15 border-red-600/30",
};

function formatTimestamp(iso: string): { date: string; time: string } {
  const d = new Date(iso);
  const date = d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
  const time = d.toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: true,
  });
  return { date, time };
}

function EntityBadge({ type }: { type: EntityType }) {
  const color = ENTITY_COLORS[type] || "#6b7280";
  return (
    <span
      className="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium"
      style={{
        backgroundColor: `${color}20`,
        color: color,
        border: `1px solid ${color}40`,
      }}
    >
      {type}
    </span>
  );
}

export default function Timeline({ events }: TimelineProps) {
  if (events.length === 0) {
    return (
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
        <p className="text-lg font-medium">No events found</p>
        <p className="mt-1 text-sm">Try adjusting the date range or filters.</p>
      </div>
    );
  }

  return (
    <div className="relative mx-auto w-full max-w-4xl">
      {/* Vertical center line */}
      <div className="absolute left-6 top-0 bottom-0 w-px bg-zinc-700 md:left-1/2 md:-translate-x-px" />

      <div className="flex flex-col gap-8 py-4">
        {events.map((event, index) => {
          const isLeft = index % 2 === 0;
          const { date, time } = formatTimestamp(event.timestamp);
          const dotColor = ENTITY_COLORS[event.entity.entity_type] || "#6b7280";
          const cardBg =
            ENTITY_BG_COLORS[event.entity.entity_type] || "bg-zinc-800/50 border-zinc-700";

          return (
            <div
              key={`${event.entity.id}-${event.timestamp}-${index}`}
              className="relative flex items-start"
            >
              {/* Mobile layout: always left-aligned with timeline on the left */}
              <div className="flex w-full md:hidden">
                {/* Dot */}
                <div className="relative z-10 flex-shrink-0">
                  <div
                    className="mt-1 h-3 w-3 rounded-full ring-4 ring-zinc-900"
                    style={{ backgroundColor: dotColor }}
                  />
                </div>

                {/* Card */}
                <div
                  className={`ml-6 flex-1 rounded-lg border p-4 ${cardBg}`}
                >
                  <div className="mb-2 flex items-center gap-2">
                    <span className="text-xs font-mono text-zinc-500">
                      {date} {time}
                    </span>
                    <EntityBadge type={event.entity.entity_type} />
                  </div>
                  <h3 className="text-sm font-semibold text-zinc-100">
                    {event.entity.name}
                  </h3>
                  <p className="mt-0.5 text-xs font-medium uppercase tracking-wider text-zinc-400">
                    {event.event_type}
                  </p>
                  <p className="mt-2 text-sm leading-relaxed text-zinc-300">
                    {event.description}
                  </p>
                  <p className="mt-2 text-xs text-zinc-500">
                    Source: {event.source}
                  </p>
                </div>
              </div>

              {/* Desktop layout: alternating left/right */}
              <div className="hidden w-full md:flex md:items-start">
                {/* Left side */}
                <div className="flex w-1/2 justify-end pr-8">
                  {isLeft ? (
                    <div
                      className={`w-full max-w-sm rounded-lg border p-4 ${cardBg}`}
                    >
                      <div className="mb-2 flex items-center justify-end gap-2">
                        <EntityBadge type={event.entity.entity_type} />
                        <span className="text-xs font-mono text-zinc-500">
                          {date}
                        </span>
                      </div>
                      <h3 className="text-right text-sm font-semibold text-zinc-100">
                        {event.entity.name}
                      </h3>
                      <p className="mt-0.5 text-right text-xs font-medium uppercase tracking-wider text-zinc-400">
                        {event.event_type}
                      </p>
                      <p className="mt-2 text-right text-sm leading-relaxed text-zinc-300">
                        {event.description}
                      </p>
                      <p className="mt-2 text-right text-xs text-zinc-500">
                        Source: {event.source}
                      </p>
                    </div>
                  ) : (
                    <div className="flex flex-col items-end justify-center pt-1">
                      <span className="text-sm font-mono font-medium text-zinc-400">
                        {date}
                      </span>
                      <span className="text-xs font-mono text-zinc-600">
                        {time}
                      </span>
                    </div>
                  )}
                </div>

                {/* Center dot */}
                <div className="relative z-10 flex-shrink-0">
                  <div
                    className="mt-1 h-3.5 w-3.5 rounded-full ring-4 ring-zinc-900"
                    style={{ backgroundColor: dotColor }}
                  />
                </div>

                {/* Right side */}
                <div className="w-1/2 pl-8">
                  {!isLeft ? (
                    <div
                      className={`w-full max-w-sm rounded-lg border p-4 ${cardBg}`}
                    >
                      <div className="mb-2 flex items-center gap-2">
                        <span className="text-xs font-mono text-zinc-500">
                          {date}
                        </span>
                        <EntityBadge type={event.entity.entity_type} />
                      </div>
                      <h3 className="text-sm font-semibold text-zinc-100">
                        {event.entity.name}
                      </h3>
                      <p className="mt-0.5 text-xs font-medium uppercase tracking-wider text-zinc-400">
                        {event.event_type}
                      </p>
                      <p className="mt-2 text-sm leading-relaxed text-zinc-300">
                        {event.description}
                      </p>
                      <p className="mt-2 text-xs text-zinc-500">
                        Source: {event.source}
                      </p>
                    </div>
                  ) : (
                    <div className="flex flex-col justify-center pt-1">
                      <span className="text-sm font-mono font-medium text-zinc-400">
                        {date}
                      </span>
                      <span className="text-xs font-mono text-zinc-600">
                        {time}
                      </span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
