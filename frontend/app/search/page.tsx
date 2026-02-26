"use client";

import { Suspense } from "react";
import Link from "next/link";
import Search from "@/components/Search";

function SearchFallback() {
  return (
    <div className="max-w-3xl mx-auto space-y-6">
      <div className="h-12 bg-white/[0.05] rounded-lg animate-pulse" />
      <div className="flex gap-2 flex-wrap">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="h-7 w-20 bg-white/[0.05] rounded-full animate-pulse" />
        ))}
      </div>
    </div>
  );
}

export default function SearchPage() {
  return (
    <div className="min-h-screen bg-[#0a0a0a]">
      {/* Header */}
      <header className="border-b border-white/[0.06] bg-white/[0.02]">
        <div className="max-w-5xl mx-auto px-6 py-4 flex items-center justify-between">
          <Link
            href="/"
            className="text-lg font-bold tracking-tight text-white hover:text-neutral-300 transition-colors"
          >
            ARGUS
          </Link>
          <nav className="flex items-center gap-4 text-sm text-neutral-500">
            <Link href="/search" className="text-neutral-200">
              Search
            </Link>
          </nav>
        </div>
      </header>

      {/* Main */}
      <main className="px-6 py-8">
        <Suspense fallback={<SearchFallback />}>
          <Search />
        </Suspense>
      </main>
    </div>
  );
}
