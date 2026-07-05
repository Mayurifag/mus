import type { Track } from "$lib/types";

export const categoryLabels: Record<string, string> = {
  "right-version": "Right version",
  "ai-cover": "AI cover",
  cover: "Cover",
  ost: "OST",
  mashup: "Mashup",
};

export const categoryNames = Object.keys(categoryLabels);

export function trackCategoryNames(track: Track): string[] {
  return (track.tags ?? [])
    .map((tag) => tag.name)
    .filter((name) => name in categoryLabels);
}

function markerText(value: string): string {
  return value
    .toLowerCase()
    .replace(/[()[\]\-_.]/g, " ")
    .split(/\s+/)
    .filter(Boolean)
    .join(" ");
}

function activeMarkerTexts(track: Track): Set<string> {
  const markers = new Set<string>();
  for (const category of trackCategoryNames(track)) {
    if (category === "right-version") {
      markers.add("right version");
      markers.add("right ver");
    } else if (category === "ai-cover") {
      markers.add("ai cover");
      markers.add("a i cover");
    } else if (category === "cover") {
      markers.add("cover");
    } else if (category === "ost") {
      markers.add("ost");
    } else if (category === "mashup") {
      markers.add("mashup");
      markers.add("mash up");
    }
  }
  return markers;
}

function stripBracketedMarkers(title: string, markers: Set<string>): string {
  const orderedMarkers = [...markers].sort((a, b) => b.length - a.length);
  return title.replace(/\s*[([]([^()[\]]+)[)\]]\s*/g, (match, value) => {
    let text = markerText(value);
    for (const marker of orderedMarkers) {
      text = text.replace(
        new RegExp(`(^| )${marker.replace(/ /g, " +")}( |$)`, "g"),
        " ",
      );
    }
    return text.trim() ? match : " ";
  });
}

function stripTrailingMarkers(title: string, markers: Set<string>): string {
  let result = title;
  let previous = "";
  while (result !== previous) {
    previous = result;
    for (const marker of markers) {
      const pattern = marker.replace(/ /g, "[\\s._-]+");
      result = result.replace(
        new RegExp(`(?:\\s*[-–—]\\s*|\\s+)${pattern}$`, "i"),
        "",
      );
    }
  }
  return result;
}

export function displayTrackTitle(track: Track): string {
  const markers = activeMarkerTexts(track);
  if (markers.size === 0) return track.title;

  const title = stripTrailingMarkers(
    stripBracketedMarkers(track.title, markers),
    markers,
  )
    .replace(/\s+/g, " ")
    .trim();
  return title || track.title;
}
