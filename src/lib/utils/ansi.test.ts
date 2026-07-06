import { describe, it, expect } from "vitest";

import { parseAnsi, stripAnsi, styleToCss } from "./ansi";

const ESC = "\x1b";

describe("stripAnsi", () => {
  it("returns plain text untouched", () => {
    expect(stripAnsi("hello world")).toBe("hello world");
  });

  it("removes SGR color codes", () => {
    expect(stripAnsi(`${ESC}[32mINFO${ESC}[39m ready`)).toBe("INFO ready");
  });

  it("removes combined SGR codes", () => {
    expect(stripAnsi(`${ESC}[1;31mERR${ESC}[0m`)).toBe("ERR");
  });

  it("removes OSC title sequences (BEL-terminated)", () => {
    expect(stripAnsi(`${ESC}]0;my title\x07body`)).toBe("body");
  });

  it("removes OSC sequences terminated by ST", () => {
    expect(stripAnsi(`${ESC}]0;title${ESC}\\body`)).toBe("body");
  });

  it("removes non-SGR CSI sequences (cursor moves)", () => {
    expect(stripAnsi(`${ESC}[2J${ESC}[Hcleared`)).toBe("cleared");
  });

  it("drops a trailing lone ESC", () => {
    expect(stripAnsi(`text${ESC}`)).toBe("text");
  });
});

describe("parseAnsi", () => {
  it("returns a single unstyled segment for plain text", () => {
    expect(parseAnsi("plain")).toEqual([{ text: "plain", style: {} }]);
  });

  it("splits styled runs into segments carrying color", () => {
    const segs = parseAnsi(`${ESC}[32mINFO${ESC}[0m done`);
    expect(segs).toHaveLength(2);
    expect(segs[0]).toEqual({ text: "INFO", style: { color: "var(--ansi-green)" } });
    expect(segs[1]).toEqual({ text: " done", style: {} });
  });

  it("maps the example log line to readable segments", () => {
    const segs = parseAnsi(
      `[11:08:06.448] ${ESC}[32mINFO${ESC}[39m (local): ${ESC}[36mEnsuring indexes${ESC}[39m`,
    );
    const joined = segs.map((s) => s.text).join("");
    expect(joined).toBe("[11:08:06.448] INFO (local): Ensuring indexes");
    const infoSeg = segs.find((s) => s.text === "INFO");
    expect(infoSeg?.style.color).toBe("var(--ansi-green)");
    const msgSeg = segs.find((s) => s.text === "Ensuring indexes");
    expect(msgSeg?.style.color).toBe("var(--ansi-cyan)");
  });

  it("handles bright foreground codes", () => {
    const segs = parseAnsi(`${ESC}[91mhot${ESC}[0m`);
    expect(segs[0].style.color).toBe("var(--ansi-bright-red)");
  });

  it("combines attributes (bold + fg)", () => {
    const segs = parseAnsi(`${ESC}[1;33mwarn${ESC}[0m`);
    expect(segs[0].style).toEqual({ bold: true, color: "var(--ansi-yellow)" });
  });

  it("supports truecolor (38;2;r;g;b)", () => {
    const segs = parseAnsi(`${ESC}[38;2;10;20;30mx${ESC}[0m`);
    expect(segs[0].style.color).toBe("rgb(10 20 30)");
  });

  it("supports 256-color palette (38;5;n)", () => {
    const segs = parseAnsi(`${ESC}[38;5;196mx${ESC}[0m`);
    expect(segs[0].style.color).toBe("rgb(255 0 0)");
  });

  it("keeps background colors", () => {
    const segs = parseAnsi(`${ESC}[44mblue${ESC}[49m`);
    expect(segs[0].style.background).toBe("var(--ansi-blue)");
  });

  it("ignores non-SGR sequences without emitting text", () => {
    const segs = parseAnsi(`a${ESC}[2J${ESC}[Hb`);
    expect(segs.map((s) => s.text).join("")).toBe("ab");
  });
});

describe("styleToCss", () => {
  it("returns undefined for an empty style", () => {
    expect(styleToCss({})).toBeUndefined();
  });

  it("emits color and weight", () => {
    expect(styleToCss({ color: "var(--ansi-red)", bold: true })).toBe(
      "font-weight:700;color:var(--ansi-red)",
    );
  });

  it("swaps fg/bg under inverse", () => {
    expect(
      styleToCss({ color: "var(--ansi-red)", background: "var(--ansi-blue)", inverse: true }),
    ).toBe("color:var(--ansi-blue);background:var(--ansi-red)");
  });

  it("combines underline and strikethrough", () => {
    expect(styleToCss({ underline: true, strikethrough: true })).toBe(
      "text-decoration:underline line-through",
    );
  });
});
