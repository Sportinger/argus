"use client";

import { useState, useRef, useEffect } from "react";
import Link from "next/link";
import type { ReasoningResponse, ReasoningStep, Entity } from "@/types/argus";

interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  response?: ReasoningResponse;
}

function ConfidenceBadge({ confidence }: { confidence: number }) {
  const pct = Math.round(confidence * 100);
  let color = "bg-red-500";
  if (pct >= 80) color = "bg-green-500";
  else if (pct >= 60) color = "bg-yellow-500";
  else if (pct >= 40) color = "bg-orange-500";

  return (
    <span className="inline-flex items-center gap-1.5 text-xs text-zinc-300">
      <span className="relative h-2 w-16 rounded-full bg-zinc-700 overflow-hidden">
        <span
          className={`absolute inset-y-0 left-0 rounded-full ${color}`}
          style={{ width: `${pct}%` }}
        />
      </span>
      {pct}% confidence
    </span>
  );
}

function ReasoningSteps({ steps }: { steps: ReasoningStep[] }) {
  const [expanded, setExpanded] = useState(false);

  if (steps.length === 0) return null;

  return (
    <div className="mt-3">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-1 text-xs text-zinc-400 hover:text-zinc-200 transition-colors"
      >
        <svg
          className={`h-3 w-3 transition-transform ${expanded ? "rotate-90" : ""}`}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
        </svg>
        Reasoning Steps ({steps.length})
      </button>
      {expanded && (
        <div className="mt-2 space-y-2 border-l-2 border-zinc-700 pl-3">
          {steps.map((step, i) => (
            <div key={i} className="text-xs">
              <p className="text-zinc-300">{step.description}</p>
              {step.cypher && (
                <pre className="mt-1 rounded bg-zinc-900 px-2 py-1 text-[11px] text-emerald-400 overflow-x-auto">
                  {step.cypher}
                </pre>
              )}
              {step.result_summary && (
                <p className="mt-1 text-zinc-500 italic">{step.result_summary}</p>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function EntityLinks({ entities }: { entities: Entity[] }) {
  if (entities.length === 0) return null;

  return (
    <div className="mt-3">
      <p className="text-xs text-zinc-500 mb-1">Referenced Entities</p>
      <div className="flex flex-wrap gap-1.5">
        {entities.map((entity) => (
          <Link
            key={entity.id}
            href={`/entity/${entity.id}`}
            className="inline-flex items-center gap-1 rounded-full bg-zinc-800 px-2.5 py-0.5 text-xs text-blue-400 hover:bg-zinc-700 hover:text-blue-300 transition-colors"
          >
            <span className="capitalize text-zinc-500">{entity.entity_type}</span>
            {entity.name}
          </Link>
        ))}
      </div>
    </div>
  );
}

function SourcesList({ sources }: { sources: string[] }) {
  if (sources.length === 0) return null;

  return (
    <div className="mt-3">
      <p className="text-xs text-zinc-500 mb-1">Sources</p>
      <ul className="list-disc list-inside text-xs text-zinc-400 space-y-0.5">
        {sources.map((source, i) => (
          <li key={i}>{source}</li>
        ))}
      </ul>
    </div>
  );
}

function LoadingIndicator() {
  return (
    <div className="flex justify-start">
      <div className="max-w-[80%] rounded-2xl rounded-bl-sm bg-zinc-700 px-4 py-3">
        <div className="flex items-center gap-2 text-sm text-zinc-300">
          <div className="flex gap-1">
            <span className="h-2 w-2 rounded-full bg-zinc-400 animate-bounce [animation-delay:-0.3s]" />
            <span className="h-2 w-2 rounded-full bg-zinc-400 animate-bounce [animation-delay:-0.15s]" />
            <span className="h-2 w-2 rounded-full bg-zinc-400 animate-bounce" />
          </div>
          Reasoning...
        </div>
      </div>
    </div>
  );
}

interface ChatProps {
  messages: ChatMessage[];
  isLoading: boolean;
}

export default function Chat({ messages, isLoading }: ChatProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, isLoading]);

  return (
    <div className="flex-1 overflow-y-auto px-4 py-6 space-y-4">
      {messages.length === 0 && !isLoading && (
        <div className="flex flex-col items-center justify-center h-full text-center text-zinc-500">
          <svg
            className="h-12 w-12 mb-4 text-zinc-700"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={1.5}
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M7.5 8.25h9m-9 3H12m-9.75 1.51c0 1.6 1.123 2.994 2.707 3.227 1.129.166 2.27.293 3.423.379.35.026.67.21.865.501L12 21l2.755-4.133a1.14 1.14 0 01.865-.501 48.172 48.172 0 003.423-.379c1.584-.233 2.707-1.626 2.707-3.228V6.741c0-1.602-1.123-2.995-2.707-3.228A48.394 48.394 0 0012 3c-2.392 0-4.744.175-7.043.513C3.373 3.746 2.25 5.14 2.25 6.741v6.018z"
            />
          </svg>
          <p className="text-lg font-medium text-zinc-400">Ask ARGUS anything</p>
          <p className="text-sm mt-1">
            Query the knowledge graph with natural language
          </p>
        </div>
      )}

      {messages.map((msg, i) => (
        <div
          key={i}
          className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
        >
          <div
            className={`max-w-[80%] rounded-2xl px-4 py-3 ${
              msg.role === "user"
                ? "rounded-br-sm bg-blue-600 text-white"
                : "rounded-bl-sm bg-zinc-700 text-zinc-100"
            }`}
          >
            <p className="text-sm whitespace-pre-wrap">{msg.content}</p>

            {msg.role === "assistant" && msg.response && (
              <div className="mt-2 border-t border-zinc-600 pt-2">
                <ConfidenceBadge confidence={msg.response.confidence} />
                <ReasoningSteps steps={msg.response.steps} />
                <EntityLinks entities={msg.response.entities_referenced} />
                <SourcesList sources={msg.response.sources} />
              </div>
            )}
          </div>
        </div>
      ))}

      {isLoading && <LoadingIndicator />}

      <div ref={bottomRef} />
    </div>
  );
}

export type { ChatMessage };
