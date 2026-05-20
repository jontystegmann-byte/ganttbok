interface ToastEntry { id: number; kind: 'error' | 'info'; message: string }

class ToastBus {
  list = $state<ToastEntry[]>([]);
  private nextId = 1;

  show(kind: 'error' | 'info', message: string, ttlMs = 4000): void {
    const id = this.nextId++;
    this.list = [...this.list, { id, kind, message }];
    setTimeout(() => {
      this.list = this.list.filter(t => t.id !== id);
    }, ttlMs);
  }
}

export const toast = new ToastBus();
