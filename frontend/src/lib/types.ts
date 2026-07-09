import { writable, type Writable } from "svelte/store";
import { Activity } from "./activity";
import { by, filterValues, mapObject, type Map } from "./mapable";
import { Part } from "./part";
import { myfetch } from "./store";
import * as m from "../../paraglide/messages";

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

  // export let types: { [key: number]: Type };
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

  activities(acts: Map<Activity>) {
    return filterValues(acts, (a) =>
      this.acts.some((t) => t.id == a.what),
    ).sort(by("start"));
  }

  parts(parts: Map<Part>) {
    return filterValues(parts, (p) => p.what == this.id).sort(by("last_used"));
  }

  human_name(hook: number | null) {
    return (
      (hook != null && this.hooks.length > 1 ? types[hook].prefix + " " : "") +
      this.name
    );
  }

  /** translated display name, falls back to the raw backend name
   * if no translation key exists for this type id */
  localizedName(): string {
    const key = `type_${this.id}`;
    const fn = (m as unknown as Record<string, () => string>)[key];
    return typeof fn === "function" ? fn() : this.name;
  }

  /** translated position prefix (front/rear/left/right etc), falls back
   * to the raw prefix if no translation key exists */
  localizedPrefix(): string {
    if (!this.prefix) return "";
    const key = `position_${this.prefix.toLowerCase()}`;
    const fn = (m as unknown as Record<string, () => string>)[key];
    return typeof fn === "function" ? fn() : this.prefix;
  }

  /** translated group name, falls back to the raw group name
   * if no translation key exists */
  localizedGroup(): string {
    if (!this.group) return "";
    const key = `group_${this.group.toLowerCase().replace(/\s+/g, "_")}`;
    const fn = (m as unknown as Record<string, () => string>)[key];
    return typeof fn === "function" ? fn() : this.group;
  }

  /** composed "position + name" label, e.g. "front tire" / "Vorderreifen",
   * used for types attached at a specific hook position */
  labelWithPosition(prefix: string): string {
    const name = this.localizedName();
    if (!prefix) return name;
    const key = `position_${prefix.toLowerCase()}`;
    const fn = (m as unknown as Record<string, () => string>)[key];
    const position = typeof fn === "function" ? fn() : prefix;
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
      types.map((t: any) => new Type(t)).reduce(mapObject("id"), {}),
    ),
    myfetch("/api/types/activity"),
  ]);

  types = activityTypes.reduce((acc: Type[], actType: ActType) => {
    acc[actType.gear_type].acts.push(actType);
    return acc;
  }, partTypes);

  category = writable(types[1]);
}

export let category: Writable<Type>;
