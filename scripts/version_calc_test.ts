import { formatVersion } from "./version_calc.ts";

function assertEqual<T>(actual: T, expected: T, msg?: string) {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) {
    throw new Error(
      `Assertion failed${msg ? `: ${msg}` : ""}\n  expected: ${e}\n  actual:   ${a}`,
    );
  }
}

Deno.test("formats version info", () => {
  assertEqual(formatVersion("0.1.2"), {
    appVersion: "0.1.2",
    releaseVersion: "0.1.2",
    releaseTag: "v0.1.2",
  });
});

Deno.test("handles prerelease versions", () => {
  assertEqual(formatVersion("1.0.0-beta.1"), {
    appVersion: "1.0.0-beta.1",
    releaseVersion: "1.0.0-beta.1",
    releaseTag: "v1.0.0-beta.1",
  });
});

Deno.test("handles major versions", () => {
  assertEqual(formatVersion("2.0.0"), {
    appVersion: "2.0.0",
    releaseVersion: "2.0.0",
    releaseTag: "v2.0.0",
  });
});
