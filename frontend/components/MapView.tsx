"use client";

import { useState, useCallback } from "react";
import DeckGL from "@deck.gl/react";
import { ScatterplotLayer } from "@deck.gl/layers";
import Map from "react-map-gl/maplibre";
import "maplibre-gl/dist/maplibre-gl.css";
import type { Entity, EntityType } from "@/types/argus";

const BASEMAP_STYLE =
  "https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json";

const INITIAL_VIEW_STATE = {
  longitude: 0,
  latitude: 20,
  zoom: 2,
  pitch: 0,
  bearing: 0,
};

const ENTITY_COLORS: Record<EntityType, [number, number, number]> = {
  person: [65, 131, 215],
  organization: [80, 200, 120],
  vessel: [0, 210, 211],
  aircraft: [240, 220, 60],
  location: [220, 50, 50],
  event: [245, 166, 35],
  document: [167, 139, 250],
  transaction: [244, 114, 182],
  sanction: [239, 68, 68],
};

function getEntityColor(type: EntityType): [number, number, number] {
  return ENTITY_COLORS[type] ?? [180, 180, 180];
}

interface TooltipInfo {
  x: number;
  y: number;
  name: string;
  type: string;
}

interface MapViewProps {
  entities: Entity[];
}

export default function MapView({ entities }: MapViewProps) {
  const [tooltip, setTooltip] = useState<TooltipInfo | null>(null);

  const entitiesWithLocation = entities.filter((e) => {
    const props = e.properties as Record<string, unknown>;
    return props?.latitude != null && props?.longitude != null;
  });

  const layers = [
    new ScatterplotLayer({
      id: "entities",
      data: entitiesWithLocation,
      pickable: true,
      opacity: 0.85,
      stroked: true,
      filled: true,
      radiusScale: 1,
      radiusMinPixels: 5,
      radiusMaxPixels: 20,
      lineWidthMinPixels: 1,
      getPosition: (d: Entity) => {
        const props = d.properties as Record<string, number>;
        return [props.longitude ?? 0, props.latitude ?? 0];
      },
      getRadius: 50000,
      getFillColor: (d: Entity) => [...getEntityColor(d.entity_type), 200],
      getLineColor: [255, 255, 255, 80],
    }),
  ];

  const onHover = useCallback(
    (info: { object?: Entity; x?: number; y?: number }) => {
      if (info.object && info.x !== undefined && info.y !== undefined) {
        setTooltip({
          x: info.x,
          y: info.y,
          name: info.object.name,
          type: info.object.entity_type,
        });
      } else {
        setTooltip(null);
      }
    },
    []
  );

  return (
    <div className="relative w-full h-full bg-zinc-950">
      <DeckGL
        initialViewState={INITIAL_VIEW_STATE}
        controller={true}
        layers={layers}
        onHover={onHover}
      >
        <Map mapStyle={BASEMAP_STYLE} />
      </DeckGL>

      {tooltip && (
        <div
          className="pointer-events-none absolute z-10 rounded-md border border-zinc-700 bg-zinc-900/95 px-3 py-2 text-sm shadow-lg backdrop-blur-sm"
          style={{
            left: tooltip.x + 12,
            top: tooltip.y + 12,
          }}
        >
          <div className="font-medium text-zinc-100">{tooltip.name}</div>
          <div className="text-xs text-zinc-400 capitalize">{tooltip.type}</div>
        </div>
      )}
    </div>
  );
}
