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
 * with Object.values/filterValues.
 */
export type StateMap<V> = Map<V> & StateMapOps<V>;

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
        if (!opKeys.has(key)) delete map[key];
      }
      apply(arr);
    },
    updateMap: (arr: V[]) => apply(arr),
    deleteItem: (id: string | number | undefined) => {
      if (id) delete map[String(id)];
    },
  };

  const opKeys = new Set(Object.keys(ops));
  return Object.assign(map, ops);
}
