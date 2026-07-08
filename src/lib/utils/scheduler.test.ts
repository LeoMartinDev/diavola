import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { debounceWithMaxWait } from "./scheduler";

describe("debounceWithMaxWait", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("coalesces calls within the wait window", () => {
    const fn = vi.fn();
    const d = debounceWithMaxWait(fn, 80, 250);
    d.schedule();
    d.schedule();
    d.schedule();
    vi.advanceTimersByTime(79);
    expect(fn).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(fn).toHaveBeenCalledOnce();
  });

  it("forces a flush after maxWait even under continuous calls", () => {
    const fn = vi.fn();
    const d = debounceWithMaxWait(fn, 80, 250);
    for (let i = 0; i < 10; i++) {
      d.schedule();
      vi.advanceTimersByTime(40);
    }
    expect(fn).toHaveBeenCalled();
  });

  it("cancel prevents a pending invocation", () => {
    const fn = vi.fn();
    const d = debounceWithMaxWait(fn, 80, 250);
    d.schedule();
    d.cancel();
    vi.advanceTimersByTime(1000);
    expect(fn).not.toHaveBeenCalled();
  });
});
