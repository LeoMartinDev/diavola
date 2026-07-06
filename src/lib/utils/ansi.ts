// ANSI escape parsing for the log viewer.
//
// Child processes frequently emit SGR (Select Graphic Rendition) color codes
// even when their stdout is piped (e.g. loggers that force colors). The raw
// bytes carry an invisible ESC (`\x1b`) byte, so without interpretation the
// viewer shows stray `[32m...[39m` fragments. This module turns those escape
// sequences into styled segments the UI can render, and provides a stripper for
// plain-text contexts (search index, clipboard copy).

const ANSI_DARK = [
  "var(--ansi-black)",
  "var(--ansi-red)",
  "var(--ansi-green)",
  "var(--ansi-yellow)",
  "var(--ansi-blue)",
  "var(--ansi-magenta)",
  "var(--ansi-cyan)",
  "var(--ansi-white)",
];
const ANSI_BRIGHT = [
  "var(--ansi-bright-black)",
  "var(--ansi-bright-red)",
  "var(--ansi-bright-green)",
  "var(--ansi-bright-yellow)",
  "var(--ansi-bright-blue)",
  "var(--ansi-bright-magenta)",
  "var(--ansi-bright-cyan)",
  "var(--ansi-bright-white)",
];

export type AnsiStyle = {
  color?: string;
  background?: string;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
  inverse?: boolean;
  strikethrough?: boolean;
};

export type AnsiSegment = {
  text: string;
  style: AnsiStyle;
};

// Matches the escape sequences we care about, in priority order:
//   1. SGR            ESC [ <params> m
//   2. other CSI      ESC [ <params> <final letter>   (cursor moves, clears…)
//   3. OSC            ESC ] … (BEL | ST=ESC \)        (titles, hyperlinks)
//   4. charset        ESC ( ) * + . / <char>
//   5. misc           ESC <char>                      (ESC c, ESC =, ESC 7…)
// `\\x1b.?` also swallows a trailing lone ESC.
const ESC_SEQ =
  /\x1b\[[0-9;]*m|\x1b\[[0-9;?]*[A-Za-z]|\x1b\](?:[^\x07]|\x1b[^\\])*(?:\x07|\x1b\\)|\x1b[()*+./].|\x1b.?/g;

function color256(n: number): string {
  if (n < 8) return ANSI_DARK[n];
  if (n < 16) return ANSI_BRIGHT[n - 8];
  if (n >= 232) {
    const v = 8 + (n - 232) * 10;
    return `rgb(${v} ${v} ${v})`;
  }
  const i = n - 16;
  const r = Math.floor(i / 36) % 6;
  const g = Math.floor(i / 6) % 6;
  const b = i % 6;
  const conv = (c: number) => (c === 0 ? 0 : 55 + c * 40);
  return `rgb(${conv(r)} ${conv(g)} ${conv(b)})`;
}

function applySgr(paramStr: string, prev: AnsiStyle): AnsiStyle {
  const params = paramStr.length === 0 ? [0] : paramStr.split(";").map((p) => Number.parseInt(p, 10) || 0);
  const s: AnsiStyle = { ...prev };
  for (let i = 0; i < params.length; i++) {
    switch (params[i]) {
      case 0:
        s.color = undefined;
        s.background = undefined;
        s.bold = undefined;
        s.dim = undefined;
        s.italic = undefined;
        s.underline = undefined;
        s.inverse = undefined;
        s.strikethrough = undefined;
        break;
      case 1: s.bold = true; break;
      case 2: s.dim = true; break;
      case 3: s.italic = true; break;
      case 4: s.underline = true; break;
      case 7: s.inverse = true; break;
      case 9: s.strikethrough = true; break;
      case 22: s.bold = undefined; s.dim = undefined; break;
      case 23: s.italic = undefined; break;
      case 24: s.underline = undefined; break;
      case 27: s.inverse = undefined; break;
      case 29: s.strikethrough = undefined; break;
      case 39: s.color = undefined; break;
      case 49: s.background = undefined; break;
      default: {
        const code = params[i];
        if (code >= 30 && code <= 37) s.color = ANSI_DARK[code - 30];
        else if (code >= 40 && code <= 47) s.background = ANSI_DARK[code - 40];
        else if (code >= 90 && code <= 97) s.color = ANSI_BRIGHT[code - 90];
        else if (code >= 100 && code <= 107) s.background = ANSI_BRIGHT[code - 100];
        else if (code === 38 || code === 48) {
          const mode = params[i + 1];
          let color: string | undefined;
          if (mode === 5) {
            color = color256(params[i + 2] ?? 0);
            i += 2;
          } else if (mode === 2) {
            const r = params[i + 2] ?? 0;
            const g = params[i + 3] ?? 0;
            const b = params[i + 4] ?? 0;
            color = `rgb(${r} ${g} ${b})`;
            i += 4;
          }
          if (code === 38) s.color = color;
          else s.background = color;
        }
      }
    }
  }
  return s;
}

export function parseAnsi(input: string): AnsiSegment[] {
  const segments: AnsiSegment[] = [];
  let style: AnsiStyle = {};
  let last = 0;
  let m: RegExpExecArray | null;
  ESC_SEQ.lastIndex = 0;
  while ((m = ESC_SEQ.exec(input)) !== null) {
    if (m.index > last) {
      segments.push({ text: input.slice(last, m.index), style: { ...style } });
    }
    const seq = m[0];
    if (seq.charCodeAt(1) === 91 /* [ */ && seq.charCodeAt(seq.length - 1) === 109 /* m */) {
      style = applySgr(seq.slice(2, -1), style);
    }
    last = ESC_SEQ.lastIndex;
  }
  if (last < input.length) {
    segments.push({ text: input.slice(last), style: { ...style } });
  }
  if (segments.length === 0 && input.length > 0) {
    segments.push({ text: input, style: {} });
  }
  return segments;
}

export function stripAnsi(input: string): string {
  ESC_SEQ.lastIndex = 0;
  return input.replace(ESC_SEQ, "");
}

export function styleToCss(style: AnsiStyle): string | undefined {
  const parts: string[] = [];
  if (style.bold) parts.push("font-weight:700");
  if (style.dim) parts.push("opacity:0.6");
  if (style.italic) parts.push("font-style:italic");
  if (style.underline || style.strikethrough) {
    const decos: string[] = [];
    if (style.underline) decos.push("underline");
    if (style.strikethrough) decos.push("line-through");
    parts.push(`text-decoration:${decos.join(" ")}`);
  }
  if (style.inverse) {
    parts.push(`color:${style.background ?? "var(--color-surface)"}`);
    parts.push(`background:${style.color ?? "var(--color-text)"}`);
  } else {
    if (style.color) parts.push(`color:${style.color}`);
    if (style.background) parts.push(`background:${style.background}`);
  }
  return parts.length > 0 ? parts.join(";") : undefined;
}
