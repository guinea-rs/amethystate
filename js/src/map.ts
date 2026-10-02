import { joined, type Path } from "./path";
import type { Transport } from "./transport";
import { same } from "./same";

/** One change to a map, as the store announces it. */
export type MapChange<V> =
  | { type: "Insert"; key: string; value: V; source: string | null }
  | { type: "Update"; key: string; oldValue: V; newValue: V; source: string | null }
  | { type: "Remove"; key: string; oldValue: V; source: string | null }
  | { type: "Clear"; source: string | null };

export type Entries<V> = ReadonlyArray<readonly [string, V]>;

type Shown<V> = { has: true; value: V } | { has: false };

function put<V>(entries: Map<string, V>, change: MapChange<V>): void {
  switch (change.type) {
    case "Insert":
      entries.set(change.key, change.value);
      break;
    case "Update":
      entries.set(change.key, change.newValue);
      break;
    case "Remove":
      entries.delete(change.key);
      break;
    case "Clear":
      entries.clear();
      break;
  }
}

/** A map stored one entry per level under its path, kept in step with the store. */
export class ReactiveMap<V> {
  readonly #held = new Map<string, V>();
  readonly #confirmed = new Map<string, V>();
  readonly #pending: MapChange<V>[] = [];
  #entries: Entries<V> | null = null;
  readonly #listeners = new Set<(entries: Entries<V>) => void>();
  readonly #keyListeners = new Map<string, Set<(value: V | undefined) => void>>();
  readonly #changeListeners = new Set<(change: MapChange<V>) => void>();
  readonly #stop: () => void;

  constructor(
    readonly path: Path,
    entries: Iterable<readonly [string, V]>,
    private readonly transport: Transport,
    private readonly source: string,
  ) {
    for (const [key, value] of entries) {
      this.#held.set(key, value);
      this.#confirmed.set(key, value);
    }
    this.#stop = transport.watch(path, (payload) => this.#hear(payload as MapChange<V>));
  }

  get(key: string): V | undefined {
    return this.#held.get(key);
  }

  has(key: string): boolean {
    return this.#held.has(key);
  }

  get size(): number {
    return this.#held.size;
  }

  /** Every entry in the order of its key; the same array until something changes. */
  entries(): Entries<V> {
    this.#entries ??= [...this.#held].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
    return this.#entries;
  }

  /** Calls `listener` with the entries now and after every change; the returned function stops it. */
  subscribe(listener: (entries: Entries<V>) => void): () => void {
    this.#listeners.add(listener);
    listener(this.entries());
    return () => {
      this.#listeners.delete(listener);
    };
  }

  /** Calls `listener` with the value under `key` now and whenever it changes or goes. */
  subscribeKey(key: string, listener: (value: V | undefined) => void): () => void {
    const listeners = this.#keyListeners.get(key) ?? new Set();
    listeners.add(listener);
    this.#keyListeners.set(key, listeners);
    listener(this.get(key));
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) this.#keyListeners.delete(key);
    };
  }

  /** Calls `listener` with each change as it happens, and nothing now. */
  onChange(listener: (change: MapChange<V>) => void): () => void {
    this.#changeListeners.add(listener);
    return () => {
      this.#changeListeners.delete(listener);
    };
  }

  /** Puts `value` under `key`, whether or not something was there. */
  insert(key: string, value: V): Promise<void> {
    const old = this.#held.get(key);
    const change: MapChange<V> = this.#held.has(key)
      ? { type: "Update", key, oldValue: old as V, newValue: value, source: this.source }
      : { type: "Insert", key, value, source: this.source };
    return this.#write(change, () => this.transport.set([...this.path, key], value, this.source));
  }

  /** Replaces the value under a `key` the map has; one it does not have is refused. */
  update(key: string, value: V): Promise<void> {
    if (!this.#held.has(key)) {
      return Promise.reject(new Error(`${joined([...this.path, key])} is not in the map`));
    }
    return this.insert(key, value);
  }

  /** Removes `key`; a key the map does not have is left alone. */
  remove(key: string): Promise<void> {
    if (!this.#held.has(key)) return Promise.resolve();
    const change: MapChange<V> = { type: "Remove", key, oldValue: this.#held.get(key) as V, source: this.source };
    return this.#write(change, () => this.transport.remove([...this.path, key], this.source));
  }

  /** Removes every entry. */
  clear(): Promise<void> {
    return this.#write({ type: "Clear", source: this.source }, () => this.transport.clear(this.path, this.source));
  }

  /** Stops watching the store; the map keeps the entries it held. */
  dispose(): void {
    this.#stop();
    this.#listeners.clear();
    this.#keyListeners.clear();
    this.#changeListeners.clear();
  }

  async #write(change: MapChange<V>, send: () => Promise<void>): Promise<void> {
    this.#pending.push(change);
    this.#apply(change);

    try {
      await send();
    } finally {
      this.#pending.splice(this.#pending.indexOf(change), 1);
      this.#rebase(this.#touchedBy(change), this.source);
    }
  }

  #hear(change: MapChange<V>): void {
    const touched = this.#touchedBy(change);
    put(this.#confirmed, change);
    if (this.#pending.length === 0 && change.source !== this.source) this.#apply(change);
    else this.#rebase(touched, change.source);
  }

  #touchedBy(change: MapChange<V>): string[] {
    if (change.type !== "Clear") return [change.key];
    return [...new Set([...this.#held.keys(), ...this.#confirmed.keys()])];
  }

  #shown(key: string): Shown<V> {
    let shown: Shown<V> = this.#confirmed.has(key)
      ? { has: true, value: this.#confirmed.get(key) as V }
      : { has: false };
    for (const write of this.#pending) {
      if (write.type === "Clear" || (write.type === "Remove" && write.key === key)) shown = { has: false };
      else if (write.type === "Insert" && write.key === key) shown = { has: true, value: write.value };
      else if (write.type === "Update" && write.key === key) shown = { has: true, value: write.newValue };
    }
    return shown;
  }

  #rebase(keys: string[], source: string | null): void {
    for (const key of keys) {
      const shown = this.#shown(key);
      const had = this.#held.has(key);
      const was = this.#held.get(key) as V;
      if (shown.has && had && !same(was, shown.value)) {
        this.#apply({ type: "Update", key, oldValue: was, newValue: shown.value, source });
      } else if (shown.has && !had) {
        this.#apply({ type: "Insert", key, value: shown.value, source });
      } else if (!shown.has && had) {
        this.#apply({ type: "Remove", key, oldValue: was, source });
      }
    }
  }

  #apply(change: MapChange<V>): void {
    const touched = change.type === "Clear" ? [...this.#held.keys(), ...this.#keyListeners.keys()] : [change.key];
    put(this.#held, change);

    this.#entries = null;
    for (const key of new Set(touched)) {
      for (const listener of [...(this.#keyListeners.get(key) ?? [])]) listener(this.get(key));
    }
    for (const listener of [...this.#changeListeners]) listener(change);
    const entries = this.entries();
    for (const listener of [...this.#listeners]) listener(entries);
  }
}
