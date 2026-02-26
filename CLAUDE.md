# ARGUS

Open Source Global Intelligence Platform — aggregates public data, extracts entities via LLM, stores in knowledge graph, queries via AI reasoning.

## Stack
- Rust (async, tokio runtime) — backend
- Neo4j (graph), Qdrant (embeddings), Claude API (reasoning/extraction)
- Next.js 16 + Deck.gl + D3 + MapLibre (frontend)
- Docker Compose for infra

## Structure (Cargo Workspace)
- `crates/argus-core/` — Shared contracts: traits, types, errors (READ-ONLY for impl crates)
- `crates/argus-agents/` — 6 data source agents (GDELT, OpenCorporates, AIS, ADS-B, OpenSanctions, EU Transparency)
- `crates/argus-graph/` — Neo4j GraphStore implementation (graceful degradation when unavailable)
- `crates/argus-extraction/` — LLM entity extraction pipeline (Claude Haiku)
- `crates/argus-reasoning/` — LLM reasoning engine (Claude Sonnet)
- `crates/argus-server/` — axum HTTP server with REST API (port 8080)
- `crates/argus-tests/` — Integration tests (115 tests)
- `frontend/` — Next.js web UI (dashboard, map, graph viz, timeline, chat, search) (port 3000)

## API Endpoints
- `GET  /api/health` — System health + Neo4j/Qdrant connectivity (3s timeout)
- `GET  /api/agents` — List all ingestion agents
- `POST /api/agents/trigger` — Trigger agent collection
- `POST /api/entities/search` — Search entities
- `GET  /api/entities/{id}` — Entity detail + neighbors
- `POST /api/graph/query` — Raw Cypher query
- `GET  /api/graph/stats` — Graph statistics
- `GET  /api/graph/neighbors/{id}` — Entity neighbor graph
- `POST /api/reasoning/query` — AI reasoning over knowledge graph
- `POST /api/timeline` — Time-ordered events

## Frontend Pages
- `/` — Dashboard (health stats, agent status, infrastructure)
- `/map` — Global map (Deck.gl ScatterplotLayer + MapLibre dark tiles)
- `/graph` — Interactive graph explorer (D3 force-directed)
- `/timeline` — Event timeline with date range picker
- `/chat` — AI chat interface for knowledge graph reasoning
- `/search` — Entity search with type filters
- `/entity/[id]` — Entity detail page

## Commands
- `docker compose up -d` — Start Neo4j + Qdrant
- `cargo build --workspace` — Build all crates
- `cargo test --workspace` — Run all tests (115 tests)
- `cargo clippy --workspace` — Lint
- `cargo run --bin argus-server` — Start backend (runs without Neo4j in degraded mode)
- `cd frontend && npm run dev` — Frontend dev server
- `cd frontend && npm run build` — Frontend production build

## Workflow
- Always commit and push after completing a task or meaningful change

## Conventions
- Serde models for all data structures
- reqwest for HTTP (async)
- async-trait for async trait definitions
- thiserror for error types
- tracing for logging
- axum for HTTP handlers with `State`, `Json`, `Path` extractors
- AGPL-3.0 license — keep it open
