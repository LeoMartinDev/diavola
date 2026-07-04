// Test setup — runs before every test file.
//
// - Loads @testing-library/jest-dom matchers (toBeInTheDocument, toBeDisabled, ...)
// - Cleans up rendered components between tests to keep them isolated.
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/svelte";
import { afterEach } from "vitest";

afterEach(() => {
  cleanup();
});
