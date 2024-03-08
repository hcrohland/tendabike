import { Activity } from "./activity";
import { Part } from "./part";
import { type Map, by, filterValues } from "./mapable";
import { types } from "./store";
export const maxDate = new Date("2999-12-31");

export class Type {
  id: number;
  name: string;
  main: number;
  hooks: Array<number>;
  order: number;
  group?: string;
  prefix: string;
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

  human_name(hook: number) {
    return (this.hooks.length > 1 ? types[hook].prefix + " " : "") + this.name;
  }
}

export type User = {
  id: number;
  firstname: string;
  name: string;
  is_admin: boolean;
};

export type ActType = {
  id: number;
  name: string;
  gear_type: number;
};
