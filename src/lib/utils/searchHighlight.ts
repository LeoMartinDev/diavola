export type TextSegment = { text: string; match: boolean };

export function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function highlightLine(text: string, query: string): TextSegment[] {
  const trimmed = query.trim();
  if (trimmed.length === 0) return [{ text, match: false }];

  const regex = new RegExp(`(${escapeRegExp(trimmed)})`, "gi");
  const segments: TextSegment[] = [];
  let lastIndex = 0;
  let m: RegExpExecArray | null;

  while ((m = regex.exec(text)) !== null) {
    if (m.index > lastIndex) {
      segments.push({ text: text.slice(lastIndex, m.index), match: false });
    }
    segments.push({ text: m[0], match: true });
    lastIndex = regex.lastIndex;
  }

  if (lastIndex < text.length) {
    segments.push({ text: text.slice(lastIndex), match: false });
  }

  return segments;
}

export function countMatches(texts: string[], query: string): number {
  const trimmed = query.trim();
  if (trimmed.length === 0) return 0;
  const regex = new RegExp(escapeRegExp(trimmed), "gi");
  let count = 0;
  for (const text of texts) {
    count += (text.match(regex) ?? []).length;
  }
  return count;
}
