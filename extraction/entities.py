"""Extract entities and relationships from raw documents using LLMs."""

from pydantic import BaseModel


class Entity(BaseModel):
    """An extracted entity (person, company, location, etc.)."""
    name: str
    type: str  # PERSON, COMPANY, LOCATION, EVENT, ASSET
    aliases: list[str] = []
    metadata: dict = {}


class Relationship(BaseModel):
    """A relationship between two entities."""
    source: str      # entity name
    target: str      # entity name
    type: str        # OWNS, FUNDS, LOCATED_IN, CONNECTED_TO, PURCHASED, etc.
    confidence: float = 1.0
    metadata: dict = {}


class ExtractionResult(BaseModel):
    """Result of entity extraction from a document."""
    entities: list[Entity] = []
    relationships: list[Relationship] = []
    raw_source: str = ""
