const ROW_HEIGHT = 22;
const OVERSCAN = 10;

export function computeVirtualScroll(scrollTop: number, viewportHeight: number, totalItems: number) {
  const totalHeight = totalItems * ROW_HEIGHT;
  const startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const endIndex = Math.min(totalItems, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN);

  return { totalHeight, startIndex, endIndex, rowHeight: ROW_HEIGHT, overscan: OVERSCAN };
}

export function isAtBottom(scrollTop: number, viewportHeight: number, scrollHeight: number, threshold = 4): boolean {
  return scrollTop + viewportHeight >= scrollHeight - threshold;
}
