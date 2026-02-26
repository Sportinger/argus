"""Base class for all ingestion agents."""

from abc import ABC, abstractmethod
from pydantic import BaseModel


class RawDocument(BaseModel):
    """A raw document fetched by an ingestion agent."""
    source: str
    url: str | None = None
    content: str
    metadata: dict = {}


class BaseAgent(ABC):
    """Base ingestion agent. Each data source implements this."""

    name: str = "base"

    @abstractmethod
    async def fetch(self) -> list[RawDocument]:
        """Fetch new documents from the source."""
        ...

    @abstractmethod
    async def health_check(self) -> bool:
        """Check if the data source is reachable."""
        ...
