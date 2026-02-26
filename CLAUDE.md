# ARGUS

Open Source Global Intelligence Platform — aggregates public data, extracts entities via LLM, stores in knowledge graph, queries via AI reasoning.

## Stack
- Rust (async, tokio runtime) — backend
- Neo4j (graph), Qdrant (embeddings), Claude API (reasoning/extraction)
- Next.js 16 + Deck.gl + D3 (frontend)
- Docker Compose for infra

## Structure (Cargo Workspace)
- `crates/argus-core/` — Shared contracts: traits, types, errors (READ-ONLY for impl crates)
- `crates/argus-agents/` — 6 data source agents (GDELT, OpenCorporates, AIS, ADS-B, OpenSanctions, EU Transparency)
- `crates/argus-graph/` — Neo4j GraphStore implementation
- `crates/argus-extraction/` — LLM entity extraction pipeline (Claude Haiku)
- `crates/argus-reasoning/` — LLM reasoning engine (Claude Sonnet)
- `crates/argus-server/` — axum HTTP server with REST API
- `crates/argus-tests/` — Integration tests
- `frontend/` — Next.js web UI (dashboard, map, graph viz, timeline, chat, search)

## Commands
- `docker compose up -d` — Start Neo4j + Qdrant
- `cargo build --workspace` — Build all crates
- `cargo test --workspace` — Run all tests
- `cargo clippy --workspace` — Lint
- `cd frontend && npm run dev` — Frontend dev server
- `cd frontend && npm run build` — Frontend production build

## Workflow
- Always commit and push after completing a task or meaningful change

## Conventions
- Serde models for all data structures
- reqwest for HTTP (async)
- Type hints everywhere, no docstrings on obvious code
- async-trait for async trait definitions
- thiserror for error types
- tracing for logging
- AGPL-3.0 license — keep it open
