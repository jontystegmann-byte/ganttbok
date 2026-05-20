export type Zone = 'move' | 'resize-start' | 'resize-end';

export function hitZone({ relX, width }: { relX: number; width: number }): Zone {
  const edge = Math.min(width * 0.1, 8);
  if (relX < edge) return 'resize-start';
  if (relX > width - edge) return 'resize-end';
  return 'move';
}
