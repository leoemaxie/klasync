import type { ConceptNode, TranscriptLine } from '$lib/api/aiStudy';

const normalise = (text: string) =>
  text
    .toLowerCase()
    .replace(/\([^)]*\)/g, ' ')
    .replace(/[^\p{L}\p{N}\s-]/gu, ' ')
    .replace(/\s+/g, ' ')
    .trim();

/** Singular-ish stem so "reaction" matches "reactions". */
const stem = (word: string) =>
  word.length > 3 && word.endsWith('s') ? word.slice(0, -1) : word;

const stemmed = (text: string) =>
  normalise(text).split(' ').map(stem).join(' ');

/**
 * Indices of the transcript lines that mention a concept by name. Matching is
 * a whole-phrase, stem-insensitive substring on word boundaries, so a concept
 * like "ATP" does not match "PATPAT" and "Calvin cycle" matches "the Calvin
 * cycles".
 */
export function findMentions(label: string, lines: TranscriptLine[]): number[] {
  const needle = stemmed(label);
  if (needle.length < 2) return [];
  const pattern = new RegExp(
    `(^|\\s)${needle.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(?=\\s|$)`
  );
  const out: number[] = [];
  lines.forEach((line, i) => {
    if (pattern.test(stemmed(line.text))) out.push(i);
  });
  return out;
}

export type ConceptMentions = {
  /** Line indices per concept id. */
  lines: Map<string, number[]>;
  /** Seconds of the first timed mention per concept id. */
  first: Map<string, number>;
};

export function indexMentions(
  nodes: ConceptNode[],
  lines: TranscriptLine[]
): ConceptMentions {
  const byNode = new Map<string, number[]>();
  const first = new Map<string, number>();
  for (const node of nodes) {
    const hits = findMentions(node.label, lines);
    byNode.set(node.id, hits);
    for (const i of hits) {
      const t = lines[i].offset_sec;
      if (t != null) {
        first.set(node.id, t);
        break;
      }
    }
  }
  return { lines: byNode, first };
}

/**
 * Concepts being discussed around `time`: any whose mention falls in the
 * window from `back` seconds before to `ahead` seconds after the playhead.
 */
export function activeConcepts(
  mentions: ConceptMentions,
  lines: TranscriptLine[],
  time: number,
  back = 14,
  ahead = 2
): Set<string> {
  const out = new Set<string>();
  for (const [id, hits] of mentions.lines) {
    for (const i of hits) {
      const t = lines[i].offset_sec;
      if (t != null && t >= time - back && t <= time + ahead) {
        out.add(id);
        break;
      }
    }
  }
  return out;
}

/** Tour order: by first mention in the lecture, then by importance. */
export function tourOrder(
  nodes: ConceptNode[],
  mentions: ConceptMentions
): string[] {
  return [...nodes]
    .sort((a, b) => {
      const ta = mentions.first.get(a.id) ?? Infinity;
      const tb = mentions.first.get(b.id) ?? Infinity;
      if (ta !== tb) return ta - tb;
      return b.importance - a.importance || a.label.localeCompare(b.label);
    })
    .map((n) => n.id);
}
