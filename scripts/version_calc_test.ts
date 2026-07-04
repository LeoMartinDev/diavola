import { computeVersion } from "./version_calc.ts";

function assertEqual<T>(actual: T, expected: T, msg?: string) {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) {
    throw new Error(
      `Assertion failed${msg ? `: ${msg}` : ""}\n  expected: ${e}\n  actual:   ${a}`,
    );
  }
}

Deno.test("first build of the day gets counter 1", () => {
  const now = new Date(2026, 5, 26); // 2026-06-26 (month is 0-indexed)
  assertEqual(computeVersion(now, []), {
    appVersion: "26.6.26-1",
    releaseVersion: "2026.06.26.1",
    releaseTag: "v2026.06.26.1",
  });
});

Deno.test("second build same day increments counter", () => {
  const now = new Date(2026, 5, 26);
  assertEqual(computeVersion(now, ["v2026.06.26.1"]), {
    appVersion: "26.6.26-2",
    releaseVersion: "2026.06.26.2",
    releaseTag: "v2026.06.26.2",
  });
});

Deno.test("ignores tags from other days", () => {
  const now = new Date(2026, 5, 26);
  assertEqual(
    computeVersion(now, ["v2026.06.25.3", "v2026.07.01.1", "v2025.12.31.9"]),
    { appVersion: "26.6.26-1", releaseVersion: "2026.06.26.1", releaseTag: "v2026.06.26.1" },
  );
});

Deno.test("ignores non-calver tags", () => {
  const now = new Date(2026, 5, 26);
  assertEqual(
    computeVersion(now, ["v0.1.0", "latest", "26.6.26-1-bogus"]),
    { appVersion: "26.6.26-1", releaseVersion: "2026.06.26.1", releaseTag: "v2026.06.26.1" },
  );
});

Deno.test("uses max counter + 1 when there are gaps", () => {
  const now = new Date(2026, 5, 26);
  assertEqual(computeVersion(now, ["v2026.06.26.1", "v2026.06.26.5"]), {
    appVersion: "26.6.26-6",
    releaseVersion: "2026.06.26.6",
    releaseTag: "v2026.06.26.6",
  });
});

Deno.test("ignores tags with malformed suffix after a valid prefix", () => {
  const now = new Date(2026, 5, 26);
  assertEqual(
    computeVersion(now, ["v2026.06.26.1-bogus", "v2026.06.26.1a"]),
    { appVersion: "26.6.26-1", releaseVersion: "2026.06.26.1", releaseTag: "v2026.06.26.1" },
  );
});

Deno.test("no leading zeros for single-digit month and day", () => {
  const now = new Date(2026, 1, 9); // 2026-02-09
  assertEqual(computeVersion(now, []), {
    appVersion: "26.2.9-1",
    releaseVersion: "2026.02.09.1",
    releaseTag: "v2026.02.09.1",
  });
});

Deno.test("computes both internal and visible release versions", () => {
  const now = new Date(2026, 6, 3); // 2026-07-03
  assertEqual(computeVersion(now, []), {
    appVersion: "26.7.3-1",
    releaseVersion: "2026.07.03.1",
    releaseTag: "v2026.07.03.1",
  });
});

Deno.test("increments the patch counter from visible release tags", () => {
  const now = new Date(2026, 6, 3);
  assertEqual(computeVersion(now, ["v2026.07.03.1"]), {
    appVersion: "26.7.3-2",
    releaseVersion: "2026.07.03.2",
    releaseTag: "v2026.07.03.2",
  });
});
