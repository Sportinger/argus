"""GDELT ingestion agent — global news events in real-time."""

import httpx
from .base import BaseAgent, RawDocument

GDELT_API = "https://api.gdeltproject.org/api/v2/doc/doc"


class GDELTAgent(BaseAgent):
    name = "gdelt"

    async def fetch(self) -> list[RawDocument]:
        async with httpx.AsyncClient() as client:
            resp = await client.get(GDELT_API, params={
                "query": "",
                "mode": "ArtList",
                "maxrecords": 250,
                "format": "json",
                "sort": "DateDesc",
            })
            resp.raise_for_status()
            data = resp.json()

        docs = []
        for article in data.get("articles", []):
            docs.append(RawDocument(
                source=self.name,
                url=article.get("url"),
                content=article.get("title", ""),
                metadata={
                    "language": article.get("language", ""),
                    "domain": article.get("domain", ""),
                    "country": article.get("sourcecountry", ""),
                    "seendate": article.get("seendate", ""),
                },
            ))
        return docs

    async def health_check(self) -> bool:
        async with httpx.AsyncClient() as client:
            resp = await client.get(GDELT_API, params={
                "query": "test",
                "mode": "ArtList",
                "maxrecords": 1,
                "format": "json",
            })
            return resp.status_code == 200
