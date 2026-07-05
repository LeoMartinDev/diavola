export type TextSegment = { text: string; match: boolean };

export type SearchOptions = { regex: boolean; caseSensitive: boolean };

export type Matcher =
  | null // query empty — no filter
  | { regex: RegExp } // valid matcher
  | { error: string }; // invalid regex — surfaced in UI

export function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function buildMatcher(
  query: string,
  options: SearchOptions,
): Matcher {
  const trimmed = query.trim();
  if (trimmed.length === 0) return null;
  const flags = options.caseSensitive ? "g" : "gi";
  const source = options.regex ? trimmed : escapeRegExp(trimmed);
  try {
    return { regex: new RegExp(source, flags) };
  } catch (err) {
    return { error: err instanceof Error ? err.message : String(err) };
  }
}

function regexOf(matcher: Matcher): RegExp | null {
  return matcher && "regex" in matcher ? matcher.regex : null;
}

export function lineMatches(matcher: Matcher, text: string): boolean {
  const re = regexOf(matcher);
  if (!re) return false;
  re.lastIndex = 0;
  return re.test(text);
}

export function highlightLine(
  text: string,
  matcher: Matcher,
): TextSegment[] {
  const re = regexOf(matcher);
  if (!re) return [{ text, match: false }];

  const segments: TextSegment[] = [];
  re.lastIndex = 0;
  let lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    if (m[0] === "") {
      // Zero-width match (e.g. `a*`): advance to avoid an infinite loop.
      re.lastIndex++;
      continue;
    }
    if (m.index > lastIndex) {
      segments.push({ text: text.slice(lastIndex, m.index), match: false });
    }
    segments.push({ text: m[0], match: true });
    lastIndex = re.lastIndex;
  }
  if (lastIndex < text.length) {
    segments.push({ text: text.slice(lastIndex), match: false });
  }
  return segments;
}
