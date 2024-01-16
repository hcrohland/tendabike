import { writable } from "svelte/store";

export type Map<V> = { [key: string]: V };

function getid<V>(v: V, field: keyof V): any {
  return v[field];
}

export function mapObject<V>(
  field: keyof V,
  del?: (v: V) => boolean,
): (a: Map<V>, b: V) => Map<V> {
  return (map, obj) => {
    if (del && del(obj)) delete map[getid(obj, field)];
    else map[getid(obj, field)] = obj;
    return map;
  };
}

export function mapable<V>(
  field: keyof V,
  prepfn?: (v: any) => V,
  delfn?: (v: V) => boolean,
) {
  let prepfn1 = prepfn || ((v) => v);
  const { subscribe, set, update } = writable<Map<V>>({});

  return {
    subscribe,
    setMap: (arr: V[]) => {
      set(arr.map(prepfn1).reduce(mapObject(field, delfn), {}));
    },
    updateMap: (arr: V[]) =>
      update((map) => arr.map(prepfn1).reduce(mapObject(field, delfn), map)),
  };
}

export function filterValues<T>(map: Map<T>, fn: (t: T) => boolean) {
  return Object.values(map).filter(fn);
}

export function by<T>(field: keyof T, asc?: boolean) {
  return (a: T, b: T) => (a[field] < b[field] ? 1 : -1) * (asc ? -1 : 1);
}
