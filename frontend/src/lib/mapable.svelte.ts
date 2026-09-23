import type { Map } from "./mapable";

/** Write operations of a state-object collection. */
export type StateMapOps<V> = {
  setMap: (arr: V[]) => void;
  updateMap: (arr: V[]) => void;
  deleteItem: (id: string | number | undefined) => void;
};

/**
 * A Svelte 5 state-object collection: a `$state` record of entities by id,
 * with the same write operations as the store-based `mapable()`. Reads of
 * the record (in any depth of plain functions) register dependencies at the
 * enclosing reactive call site; writes through the operations are in-place.
 *
 * The operations live on the record itself, so `Object.keys`/`Object.values`
 * over the collection also see them — do not iterate a state collection
 * with Object.values/filterValues; use `stateValues` instead.
 */
export type StateMap<V> = Map<V> & StateMapOps<V>;

/** Keys of the write operations attached to a state collection. Any
 * enumeration of the record (Object.keys/entries/values) also sees them;
 * skip these keys when collecting the entity values. */
export const stateMapOpKeys = new Set(["setMap", "updateMap", "deleteItem"]);

/**
 * The entity values of a collection, in record order: enumerates the
 * record and skips the keys of the write operations attached by
 * `mapableState`. This is the way to enumerate a state collection —
 * `Object.values`/`filterValues` would treat the operations as entries.
 * On a plain record without attached operations it is a plain `Object.values`.
 * The state overload is listed first so the value type infers from the
 * `StateMap` constituents, not from the operation properties.
 */
export function stateValues<V>(map: StateMap<V>): V[];
export function stateValues<V>(map: Map<V>): V[];
export function stateValues<V>(map: Map<V>): V[] {
  // SAFETY: values under the operation keys are filtered out above; the
  // remaining values are the record's entities.
  return Object.entries(map)
    .filter(([key]) => !stateMapOpKeys.has(key))
    .map(([, value]) => value);
}

function getid<V>(v: V, field: keyof V): any {
  return v[field];
}

/**
 * State-object twin of `mapable()` in mapable.ts: produces a `$state`
 * record instead of a svelte/store writable. `setMap` replaces the whole
 * record, `updateMap` merges into it, `deleteItem` removes one entry.
 */
export function mapableState<V>(
  field: keyof V,
  prepfn?: (v: any) => V,
  delfn?: (v: V) => boolean,
): StateMap<V> {
  let prepfn1 = prepfn || ((v) => v);
  const map: Map<V> = $state({});

  const apply = (arr: V[]) => {
    for (const raw of arr) {
      const v = prepfn1(raw);
      const id = String(getid(v, field));
      if (delfn && delfn(v)) delete map[id];
      else map[id] = v;
    }
  };

  const ops: StateMapOps<V> = {
    setMap: (arr: V[]) => {
      for (const key of Object.keys(map)) {
        if (!stateMapOpKeys.has(key)) delete map[key];
      }
      apply(arr);
    },
    updateMap: (arr: V[]) => apply(arr),
    deleteItem: (id: string | number | undefined) => {
      if (id) delete map[String(id)];
    },
  };

  return Object.assign(map, ops);
}
