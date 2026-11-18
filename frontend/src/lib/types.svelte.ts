import { Activity, activities as allActivities } from "./activity";
import { by, filterValues, type Map, stateValues } from "./mapable.svelte";
import { Part, parts as allParts } from "./part";
import { myfetch } from "./utils";
import * as m from "../../paraglide/messages";

// Translate a paraglide message key, falling back to `fallback` when the key
// is missing or not a function.
// SAFETY: the paraglide module exposes one function per compiled message key;
// unknown keys are checked below and fall back.
function translate(mod: typeof m, key: string, fallback: string): string {
  const fn = (mod as unknown as Record<string, () => string>)[key];
  return typeof fn === "function" ? fn() : fallback;
}

export class Type {
  id: number;
  name: string;
  // the main gear type
  main: number;
  // the types this type can be attached to
  hooks: Array<number>;
  // the order in which the types are displayed
  order: number;
  // the group for the setup wizard
  group?: string;
  // the position name for types which can be attached to multiple positions
  prefix: string;
  // the activity types this type can use
  acts: ActType[];

  constructor(t: any) {
    this.id = t.id;
    this.name = t.name;
    this.main = t.main;
    this.hooks = t.hooks;
    this.order = t.order;
    this.group = t.group;

    this.prefix = this.name.split(" ").reverse()[1] || ""; // The first word iff there were two (hack!)
    // will be populated by getTypes()
    this.acts = [];
  }

  activities(): Activity[] {
    return stateValues(allActivities)
      .filter((a) => this.acts.some((t) => t.id == a.what))
      .sort(by("start"));
  }

  parts(): Part[] {
    return stateValues(allParts)
      .filter((p) => p.what == this.id)
      .sort(by("last_used"));
  }

  human_name(hook: number | null) {
    if (hook != null && this.hooks.length > 1) {
      return this.labelWithPosition(types[hook].prefix);
    }
    return this.localizedName();
  }

  /** translated display name, falls back to the raw backend name
   * if no translation key exists for this type id */
  localizedName(): string {
    return translate(m, `type_${this.id}`, this.name);
  }

  /** translated position prefix (front/rear/left/right etc), falls back
   * to the raw prefix if no translation key exists */
  localizedPrefix(): string {
    if (!this.prefix) return "";
    return translate(m, `position_${this.prefix.toLowerCase()}`, this.prefix);
  }

  /** translated group name, falls back to the raw group name
   * if no translation key exists */
  localizedGroup(): string {
    return localizeGroupName(this.group);
  }

  /** composed "position + name" label, e.g. "front tire" / "Vorderreifen",
   * used for types attached at a specific hook position */
  labelWithPosition(prefix: string): string {
    const name = this.localizedName();
    if (!prefix) return name;
    const position = translate(m, `position_${prefix.toLowerCase()}`, prefix);
    return m.type_with_position({ position, name });
  }

  subtypes() {
    return filterValues(types, (t) => t.main == this.id && t.id != t.main).sort(
      (a, b) => a.order - b.order,
    );
  }

  is_hook() {
    return filterValues(types, (t) => t.hooks.includes(this.id)).length > 0;
  }

  localizedAnyNominative(): string {
    return translate(m, `category_any_nom_${this.id}`, `any ${this.name}`);
  }

  localizedAnyDative(): string {
    return translate(m, `category_any_dat_${this.id}`, `any ${this.name}`);
  }

  localizedOldAccusative(): string {
    return translate(m, `type_old_acc_${this.id}`, `old ${this.name}`);
  }
}

export type ActType = {
  id: number;
  name: string;
  gear_type: number;
};

export let types: Map<Type>;

export async function getTypes() {
  const [partTypes, activityTypes] = await Promise.all([
    myfetch("/api/types/part").then((types) =>
      types
        .map((t: any) => new Type(t))
        .reduce((map: Map<Type>, t: Type) => {
          map[t.id] = t;
          return map;
        }, {}),
    ),
    myfetch("/api/types/activity"),
  ]);

  types = activityTypes.reduce((acc: Type[], actType: ActType) => {
    acc[actType.gear_type].acts.push(actType);
    return acc;
  }, partTypes);

  setCategory(types[1]);
}

export function localizeGroupName(group: string | undefined): string {
  if (!group) return "";
  return translate(
    m,
    `group_${group.toLowerCase().replace(/\s+/g, "_")}`,
    group,
  );
}

/**
 * The active gear category, as a Svelte 5 state object. The value is
 * nullable (unset until getTypes() resolves) and replaced wholesale, which
 * a `.svelte.ts` module may not export directly (Svelte's
 * `state_invalid_export` rule), so the state lives in the module and is
 * read via `getCategory` and replaced via `setCategory`. Reads register
 * dependencies at the enclosing reactive call site.
 */
let category = $state<Type | undefined>(undefined);

export function getCategory(): Type | undefined {
  return category;
}

/** Set the active category to `value`; `undefined` clears it. */
export function setCategory(value: Type | undefined) {
  category = value;
}
