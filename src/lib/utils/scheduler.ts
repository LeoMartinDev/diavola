type Schedule = {
  schedule: () => void;
  cancel: () => void;
};

export function debounceWithMaxWait(
  fn: () => void,
  wait: number,
  maxWait: number,
): Schedule {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let maxTimer: ReturnType<typeof setTimeout> | null = null;

  function invoke(): void {
    cancel();
    fn();
  }

  function cancel(): void {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
    if (maxTimer !== null) {
      clearTimeout(maxTimer);
      maxTimer = null;
    }
  }

  function schedule(): void {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(invoke, wait);
    if (maxTimer === null) {
      maxTimer = setTimeout(invoke, maxWait);
    }
  }

  return { schedule, cancel };
}
