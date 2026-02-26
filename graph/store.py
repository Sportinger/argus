"""Neo4j graph store — persist entities and relationships."""

from neo4j import AsyncGraphDatabase
from extraction.entities import Entity, Relationship


class GraphStore:
    def __init__(self, uri: str, user: str, password: str):
        self.driver = AsyncGraphDatabase.driver(uri, auth=(user, password))

    async def close(self):
        await self.driver.close()

    async def upsert_entity(self, entity: Entity):
        query = """
        MERGE (e:Entity {name: $name})
        SET e.type = $type,
            e.aliases = $aliases,
            e += $metadata
        """
        async with self.driver.session() as session:
            await session.run(query,
                name=entity.name,
                type=entity.type,
                aliases=entity.aliases,
                metadata=entity.metadata,
            )

    async def upsert_relationship(self, rel: Relationship):
        query = """
        MATCH (a:Entity {name: $source})
        MATCH (b:Entity {name: $target})
        MERGE (a)-[r:RELATED {type: $type}]->(b)
        SET r.confidence = $confidence,
            r += $metadata
        """
        async with self.driver.session() as session:
            await session.run(query,
                source=rel.source,
                target=rel.target,
                type=rel.type,
                confidence=rel.confidence,
                metadata=rel.metadata,
            )

    async def query(self, cypher: str, params: dict = {}):
        async with self.driver.session() as session:
            result = await session.run(cypher, params)
            return [record.data() async for record in result]
