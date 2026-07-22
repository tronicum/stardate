/** Binary max-heap keyed by a caller-supplied score function. */
export class MaxHeap<T> {
  private items: T[] = [];

  constructor(private score: (item: T) => number) {}

  push(item: T): void {
    this.items.push(item);
    let i = this.items.length - 1;
    while (i > 0) {
      const parent = (i - 1) >> 1;
      if (this.score(this.items[parent]) >= this.score(this.items[i])) break;
      [this.items[parent], this.items[i]] = [this.items[i], this.items[parent]];
      i = parent;
    }
  }

  pop(): T | undefined {
    if (this.items.length === 0) return undefined;
    const top = this.items[0];
    const last = this.items.pop() as T;
    if (this.items.length > 0) {
      this.items[0] = last;
      this.sinkDown(0);
    }
    return top;
  }

  private sinkDown(start: number): void {
    let i = start;
    const n = this.items.length;
    for (;;) {
      const l = 2 * i + 1;
      const r = 2 * i + 2;
      let largest = i;
      if (l < n && this.score(this.items[l]) > this.score(this.items[largest])) largest = l;
      if (r < n && this.score(this.items[r]) > this.score(this.items[largest])) largest = r;
      if (largest === i) break;
      [this.items[i], this.items[largest]] = [this.items[largest], this.items[i]];
      i = largest;
    }
  }

  get size(): number {
    return this.items.length;
  }
}
