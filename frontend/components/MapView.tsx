"use client";

import { useState, useMemo, useCallback } from "react";
import MapGL, { Source, Layer, Popup } from "react-map-gl/maplibre";
import type { MapLayerMouseEvent } from "react-map-gl/maplibre";
import type { CircleLayerSpecification } from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import type { Entity, EntityType } from "@/types/argus";

const BASEMAP_STYLE =
  "https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json";

const ENTITY_COLORS: Record<EntityType, string> = {
  person: "#4183d7",
  organization: "#50c878",
  vessel: "#00d2d3",
  aircraft: "#f0dc3c",
  location: "#dc3232",
  event: "#f5a623",
  document: "#a78bfa",
  transaction: "#f472b6",
  sanction: "#ef4444",
};

interface MapViewProps {
  entities: Entity[];
}

export default function MapView({ entities }: MapViewProps) {
  const [popupInfo, setPopupInfo] = useState<{
    lng: number;
    lat: number;
    name: string;
    type: string;
  } | null>(null);

  const geojson = useMemo(() => {
    const features = entities
      .filter((e) => {
        const props = e.properties as Record<string, unknown>;
        return props?.latitude != null && props?.longitude != null;
      })
      .map((e) => {
        const props = e.properties as Record<string, number>;
        return {
          type: "Feature" as const,
          geometry: {
            type: "Point" as const,
            coordinates: [props.longitude, props.latitude],
          },
          properties: {
            name: e.name,
            entityType: e.entity_type,
            color: ENTITY_COLORS[e.entity_type] ?? "#b4b4b4",
          },
        };
      });

    return {
      type: "FeatureCollection" as const,
      features,
    };
  }, [entities]);

  const circleLayer: CircleLayerSpecification = {
    id: "entity-points",
    type: "circle",
    source: "entities",
    paint: {
      "circle-radius": 6,
      "circle-color": ["get", "color"],
      "circle-opacity": 0.85,
      "circle-stroke-width": 1,
      "circle-stroke-color": "rgba(255,255,255,0.3)",
    },
  };

  const onClick = useCallback((event: MapLayerMouseEvent) => {
    const feature = event.features?.[0];
    if (feature && feature.geometry.type === "Point") {
      const [lng, lat] = feature.geometry.coordinates;
      setPopupInfo({
        lng,
        lat,
        name: feature.properties?.name ?? "Unknown",
        type: feature.properties?.entityType ?? "unknown",
      });
    }
  }, []);

  return (
    <div className="relative w-full h-full">
      <MapGL
        initialViewState={{
          longitude: 0,
          latitude: 20,
          zoom: 2,
        }}
        style={{ width: "100%", height: "100%" }}
        mapStyle={BASEMAP_STYLE}
        interactiveLayerIds={["entity-points"]}
        onClick={onClick}
        cursor="auto"
      >
        <Source id="entities" type="geojson" data={geojson}>
          <Layer {...circleLayer} />
        </Source>

        {popupInfo && (
          <Popup
            longitude={popupInfo.lng}
            latitude={popupInfo.lat}
            anchor="bottom"
            onClose={() => setPopupInfo(null)}
            closeButton={true}
            className="[&_.maplibregl-popup-content]:!bg-zinc-900 [&_.maplibregl-popup-content]:!text-zinc-100 [&_.maplibregl-popup-content]:!border-zinc-700 [&_.maplibregl-popup-content]:!border [&_.maplibregl-popup-content]:!rounded-md [&_.maplibregl-popup-content]:!shadow-lg [&_.maplibregl-popup-tip]:!border-t-zinc-900"
          >
            <div className="font-medium text-sm">{popupInfo.name}</div>
            <div className="text-xs text-zinc-400 capitalize">{popupInfo.type}</div>
          </Popup>
        )}
      </MapGL>
    </div>
  );
}
