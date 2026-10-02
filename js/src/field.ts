import { joined, type Path } from "./path";
import type { Transport } from "./transport";
import { same } from "./same";

/** What the store says about a field: the value it now holds, or that its key went, with the default the field takes where it takes one. */
export type FieldChange<T> =
  | { type: "Value"; value: T; source: string | null }
  | { type: "Deleted"; value?: T; source: string | null };

/** One stored value, kept in step with the store. */
export class Field<T> {
  #value: T;
  #confirmed: T;
  readonly #pending: { value: T }[] = [];
  readonly #listeners = new Set<(value: T) => void>();
  readonly #stop: () => void;

  constructor(
    readonly path: Path,
    value: T,
    private readonly transport: Transport,
    private readonly source: string,
  ) {
    this.#value = value;
    this.#confirmed = value;
    this.#stop = transport.watch(path, (payload) => this.#hear(payload as FieldChange<T>));
  }

  /** What the field holds now. */
  get(): T {
    return this.#value;
  }

  /** Calls `listener` with the value now and on every change; the returned function stops it. */
  subscribe(listener: (value: T) => void): () => void {
    this.#listeners.add(listener);
    listener(this.#value);
    return () => {
      this.#listeners.delete(listener);
    };
  }

  /** Takes `value` at once and writes it; a write the store refuses is taken back, and the promise rejects. */
  async set(value: T): Promise<void> {
    const write = { value };
    this.#pending.push(write);
    this.#take(value);

    try {
      await this.transport.set(this.path, value, this.source);
    } finally {
      this.#pending.splice(this.#pending.indexOf(write), 1);
      this.#show();
    }
  }

  /** Sets what `change` makes of the current value. */
  update(change: (value: T) => T): Promise<void> {
    return this.set(change(this.#value));
  }

  /** Stops watching the store; the field keeps the last value it held. */
  dispose(): void {
    this.#stop();
    this.#listeners.clear();
  }

  toString(): string {
    return `Field(${joined(this.path)})`;
  }

  #hear(change: FieldChange<T>): void {
    if (change.type === "Value") this.#confirmed = change.value;
    else if ("value" in change) this.#confirmed = change.value as T;
    this.#show();
  }

  #show(): void {
    const last = this.#pending.at(-1);
    this.#take(last ? last.value : this.#confirmed);
  }

  #take(value: T): void {
    if (same(this.#value, value)) return;
    this.#value = value;
    for (const listener of [...this.#listeners]) listener(value);
  }
}
