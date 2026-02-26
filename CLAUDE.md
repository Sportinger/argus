# ARGUS

Open Source Global Intelligence Platform — aggregates public data, extracts entities via LLM, stores in knowledge graph, queries via AI reasoning.

## Stack
- Rust (async, tokio runtime) — backend
- Neo4j (graph), Qdrant (embeddings), Claude API (reasoning/extraction)
- Next.js 16 + MapLibre GL + D3 (frontend)
- Docker Compose for infra (Neo4j, Qdrant)

## Structure (Cargo Workspace)
- `crates/argus-core/` — Shared contracts: traits, types, errors (READ-ONLY for impl crates)
- `crates/argus-agents/` — 6 data source agents (GDELT, OpenCorporates, AIS, ADS-B, OpenSanctions, EU Transparency)
- `crates/argus-graph/` — Neo4j GraphStore implementation (graceful degradation, 5s timeout on all ops)
- `crates/argus-extraction/` — LLM entity extraction pipeline (Claude Haiku)
- `crates/argus-reasoning/` — LLM reasoning engine (Claude Sonnet)
- `crates/argus-server/` — axum HTTP server with REST API (port 8080)
- `crates/argus-tests/` — Integration tests
- `frontend/` — Next.js web UI (dashboard, map, graph viz, timeline, chat, search) (port 3000)

## API Endpoints
- `GET  /api/health` — System health + Neo4j/Qdrant connectivity
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
- `/map` — Global map (MapLibre GL with CARTO dark basemap, GeoJSON circle layers)
- `/graph` — Interactive graph explorer (D3 force-directed)
- `/timeline` — Event timeline with date range picker
- `/chat` — AI chat interface for knowledge graph reasoning
- `/search` — Entity search with type filters
- `/entity/[id]` — Entity detail page

## Commands
- `docker compose up -d neo4j qdrant` — Start databases
- `cargo build --workspace` — Build all crates
- `cargo test --workspace` — Run all tests
- `cargo clippy --workspace` — Lint
- `cargo run --bin argus-server` — Start backend (graceful degradation without Neo4j)
- `cd frontend && npm run dev` — Frontend dev server
- `cd frontend && npm run build` — Frontend production build

## Defaults
- Neo4j: `bolt://localhost:7687`, user `neo4j`, password `argus2026`
- Qdrant: `http://localhost:6333`
- Backend: `0.0.0.0:8080`
- Frontend: `localhost:3000`

## Workflow
- Always commit and push after completing a task or meaningful change
- Always stop old server processes before starting new ones

## Conventions
- Serde models for all data structures
- reqwest for HTTP (async)
- async-trait for async trait definitions
- thiserror for error types
- tracing for logging
- axum for HTTP handlers with `State`, `Json`, `Path` extractors
- All Neo4j operations wrapped with 5s timeout (`timed()` in store.rs)
- AGPL-3.0 license — keep it open
