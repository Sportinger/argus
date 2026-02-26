# ARGUS

**Open Source Global Intelligence Platform — Palantir for the People.**

ARGUS aggregates public data streams worldwide and uses AI to uncover hidden connections — making the same insights available to everyone that were previously reserved for intelligence agencies and hedge funds.

## What it does

- Ingests public data sources 24/7 (news, corporate registries, ship/flight tracking, satellite imagery, financial filings, lobby registers)
- Extracts entities (people, companies, locations, events) and relationships using LLMs
- Stores everything in a knowledge graph (Neo4j)
- Enables natural language queries via AI reasoning layer
- Visualizes connections on interactive maps, graphs, and timelines

## Architecture

```
Frontend (Next.js + Deck.gl)
        │
Reasoning Layer (Claude / GPT API)
        │
Knowledge Graph (Neo4j) + Embeddings (Qdrant)
        │
Ingestion Agents (Python, 24/7)
        │
Public Data Sources (GDELT, OpenCorporates, AIS, ADS-B, Sentinel, ...)
```

## Project Structure

```
argus/
├── agents/          # Data ingestion agents (one per source)
├── extraction/      # Entity & relationship extraction pipeline
├── graph/           # Neo4j graph layer & queries
├── reasoning/       # LLM reasoning & query engine
├── frontend/        # Web UI (map, graph, timeline, chat)
├── config/          # Configuration & source definitions
├── data/            # Local data cache
└── docs/            # Documentation & architecture notes
```

## Data Sources

| Source | Data | Status |
|---|---|---|
| GDELT | Global news events, real-time | Planned |
| OpenCorporates | 200M+ companies worldwide | Planned |
| AIS Exchange | Global ship tracking | Planned |
| ADS-B Exchange | Global flight tracking | Planned |
| Sentinel Hub | Satellite imagery (ESA) | Planned |
| EU Transparency Register | Lobby spending EU | Planned |
| OpenSanctions | Sanctions lists, PEPs | Planned |
| Offene Register | German company registry | Planned |
| OCCRP Aleph | Investigative database | Planned |
| World Bank Open Data | Development & economic data | Planned |

## Tech Stack

- **Backend:** Python 3.12+
- **Graph DB:** Neo4j
- **Vector DB:** Qdrant
- **LLM (extraction):** Llama (local) or Claude Haiku
- **LLM (reasoning):** Claude API
- **Frontend:** Next.js, Deck.gl, D3.js
- **Infrastructure:** Docker, self-hosted (Hetzner)

## License

AGPL-3.0 — Free forever. If you build on this, you share back.
