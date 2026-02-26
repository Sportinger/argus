# ARGUS

Open Source Global Intelligence Platform — aggregates public data, extracts entities via LLM, stores in knowledge graph, queries via AI reasoning.

## Stack
- Python 3.12+, async everywhere
- Neo4j (graph), Qdrant (embeddings), Claude API (reasoning), Haiku/Llama (extraction)
- Next.js + Deck.gl (frontend)
- Docker Compose for infra

## Structure
- `agents/` — Ingestion agents, one per data source. Extend `BaseAgent` in `base.py`
- `extraction/` — Entity/relationship extraction pipeline using LLMs
- `graph/` — Neo4j store, Cypher queries
- `reasoning/` — LLM query engine over the graph
- `frontend/` — Web UI (map, graph viz, timeline, chat)
- `config/sources.yaml` — All data source definitions

## Commands
- `docker compose up -d` — Start Neo4j + Qdrant
- `pip install -e .` — Install project
- `pip install -e ".[dev]"` — Install with dev deps
- `pytest` — Run tests

## Conventions
- Pydantic models for all data structures
- httpx for HTTP (async)
- Type hints everywhere, no docstrings on obvious code
- AGPL-3.0 license — keep it open
