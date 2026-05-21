export type Zone = 'move' | 'resize-start' | 'resize-end';

/** Wider edge zone (~9px or 15%) so resize handles are easy to grab. */
export const EDGE_PX = 9;

export function hitZone({ relX, width }: { relX: number; width: number }): Zone {
  const edge = Math.min(Math.max(width * 0.15, EDGE_PX), 14);
  if (relX < edge) return 'resize-start';
  if (relX > width - edge) return 'resize-end';
  return 'move';
}
