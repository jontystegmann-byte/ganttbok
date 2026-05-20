export type Zone = 'move' | 'resize-start' | 'resize-end';

export function hitZone({ relX, width }: { relX: number; width: number }): Zone {
  // Edge zone is 10% of bar width, with a 4px floor so narrow bars remain grabbable on both ends.
  const edge = Math.max(width * 0.1, 4);
  if (relX < edge) return 'resize-start';
  if (relX > width - edge) return 'resize-end';
  return 'move';
}
