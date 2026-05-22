/**
 * inbox-diff.ts
 *
 * Pure function that converts a single PatchOp into a human-readable
 * one-line diff string for display in the Inbox panel.
 * No Svelte, no store, no side effects.
 */
import type { PatchOp, Phase, Task, Contact } from './types';

export function renderPatchOp(
  op: PatchOp,
  phases: Phase[],
  tasks: Task[],
  contacts: Contact[],
): string {
  const taskName = (id: number) =>
    tasks.find((t) => t.id === id)?.name ?? `task #${id}`;

  const phaseName = (id: number) =>
    phases.find((p) => p.id === id)?.name ?? `phase #${id}`;

  const contactName = (id: number) =>
    contacts.find((c) => c.id === id)?.name ?? `contact #${id}`;

  const taskRefLabel = (ref: { task_id: number } | { op_ref: string }): string => {
    if ('task_id' in ref) return taskName(ref.task_id);
    return `(new: ${ref.op_ref})`;
  };

  switch (op.op) {
    case 'add_task': {
      const phase = phaseName(op.phase_id);
      const contact = op.contact_id != null ? ` (assigned: ${contactName(op.contact_id)})` : '';
      return `+ Add "${op.name}" to ${phase}, starts ${op.start_date}, ${op.duration_workdays}d${contact}`;
    }

    case 'shift_task': {
      const sign = op.by_days >= 0 ? '+' : '';
      return `↻ Shift "${taskName(op.task_id)}" by ${sign}${op.by_days} workdays`;
    }

    case 'add_dependency': {
      const pred = taskRefLabel(op.predecessor);
      const succ = taskRefLabel(op.successor);
      const lag = (op.lag_days ?? 0) !== 0 ? ` (lag: ${op.lag_days}d)` : '';
      const type_ = op.dep_type ?? 'FS';
      return `→ Add dependency: "${pred}" ${type_} "${succ}"${lag}`;
    }

    case 'add_chaser': {
      return `🔔 Chaser "${taskName(op.task_id)}" → ${contactName(op.contact_id)} (${op.template})`;
    }

    case 'append_note': {
      const preview = op.text.length > 80 ? op.text.slice(0, 77) + '…' : op.text;
      return `📝 Note: "${preview}"`;
    }

    default: {
      // Exhaustiveness guard — TypeScript will warn if a new op is added.
      const _: never = op;
      return `Unknown op`;
    }
  }
}
