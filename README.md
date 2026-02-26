# ARGUS

**Open Source Global Intelligence Platform — Palantir for the People.**

ARGUS aggregates public data streams worldwide and uses AI to uncover hidden connections — making the same insights available to everyone that were previously reserved for intelligence agencies and hedge funds.

## What it does

- Ingests public data sources 24/7 (news, corporate registries, ship/flight tracking, sanctions lists, lobby registers)
- Extracts entities (people, companies, locations, events) and relationships using LLMs
- Stores everything in a knowledge graph (Neo4j) with vector embeddings (Qdrant)
- Enables natural language queries via AI reasoning layer
- Visualizes connections on interactive maps, graphs, and timelines

## Architecture

```
Frontend (Next.js + MapLibre + D3)
        │
   REST API (axum, Rust)
        │
Reasoning Layer (Claude Sonnet)
        │
Knowledge Graph (Neo4j) + Embeddings (Qdrant)
        │
Extraction Pipeline (Claude Haiku)
        │
Ingestion Agents (Rust, async)
        │
Public Data Sources (GDELT, OpenCorporates, AIS, ADS-B, OpenSanctions, ...)
```

## Project Structure

```
argus/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── argus-core/               # Shared traits, types, errors
│   ├── argus-agents/             # 6 data source agents
│   ├── argus-graph/              # Neo4j graph store
│   ├── argus-extraction/         # LLM entity extraction
│   ├── argus-reasoning/          # LLM reasoning engine
│   ├── argus-server/             # axum HTTP server
│   └── argus-tests/              # Integration tests
├── frontend/                     # Next.js web UI
├── docker-compose.yml            # Neo4j + Qdrant
├── Dockerfile                    # Multi-stage build
└── .github/workflows/ci.yml     # CI pipeline
```

## Data Sources

| Source | Data | Agent |
|---|---|---|
| GDELT | Global news events, real-time | `gdelt.rs` |
| OpenCorporates | 200M+ companies worldwide | `opencorporates.rs` |
| AIS Hub | Global ship tracking | `ais.rs` |
| ADS-B Exchange | Global flight tracking | `adsb.rs` |
| OpenSanctions | Sanctions lists, PEPs | `opensanctions.rs` |
| EU Transparency Register | Lobby spending EU | `eu_transparency.rs` |

## Quick Start

```bash
# 1. Start databases
docker compose up -d neo4j qdrant

# 2. Build and run backend
cargo run --bin argus-server

# 3. Start frontend (separate terminal)
cd frontend && npm install && npm run dev

# 4. Open browser
# Dashboard: http://localhost:3000
# API:       http://localhost:8080/api/health
```

## API

| Method | Endpoint | Description |
|---|---|---|
| GET | `/api/health` | System health + connectivity |
| GET | `/api/agents` | List ingestion agents |
| POST | `/api/agents/trigger` | Trigger agent data collection |
| POST | `/api/entities/search` | Search entities by name/type |
| GET | `/api/entities/{id}` | Entity detail with neighbors |
| POST | `/api/graph/query` | Raw Cypher query |
| GET | `/api/graph/stats` | Graph statistics |
| GET | `/api/graph/neighbors/{id}` | Entity neighbor subgraph |
| POST | `/api/reasoning/query` | AI reasoning over knowledge graph |
| POST | `/api/timeline` | Time-ordered entity events |

## Frontend

| Page | Description |
|---|---|
| `/` | Dashboard — health, agent status, infrastructure overview |
| `/map` | Global map — entities plotted on MapLibre dark basemap |
| `/graph` | Graph explorer — D3 force-directed interactive visualization |
| `/timeline` | Timeline — chronological entity events with filters |
| `/chat` | AI chat — natural language queries over the knowledge graph |
| `/search` | Entity search — full-text search with type filters |
| `/entity/[id]` | Entity detail — properties, relationships, neighbor graph |

## Tech Stack

- **Backend:** Rust (tokio, axum, reqwest, async-trait)
- **Graph DB:** Neo4j 5 Community
- **Vector DB:** Qdrant
- **LLM (extraction):** Claude Haiku
- **LLM (reasoning):** Claude Sonnet
- **Frontend:** Next.js 16, MapLibre GL, D3.js, Tailwind CSS
- **Infrastructure:** Docker Compose, GitHub Actions CI

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `NEO4J_URI` | `bolt://localhost:7687` | Neo4j connection URI |
| `NEO4J_USER` | `neo4j` | Neo4j username |
| `NEO4J_PASSWORD` | `argus2026` | Neo4j password |
| `QDRANT_URL` | `http://localhost:6333` | Qdrant URL |
| `ANTHROPIC_API_KEY` | — | Required for extraction + reasoning |
| `SERVER_HOST` | `0.0.0.0` | Backend bind host |
| `SERVER_PORT` | `8080` | Backend bind port |

## License

AGPL-3.0 — Free forever. If you build on this, you share back.
