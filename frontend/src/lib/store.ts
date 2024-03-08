import { writable, type Writable } from "svelte/store";
import { mapObject } from "./mapable";
import { Type, type ActType, type User, maxDate } from "./types";
import { Service, services } from "./service";
import { activities, Activity } from "../lib/activity";
import { Usage, usages } from "../Usage/usage";
import { parts, type Part } from "../lib/part";
import { Attachment, attachments } from "./attachment";
import { plans, type ServicePlan } from "./serviceplan";

export const DAY = 24 * 60 * 60 * 1000;

export function get_days(start: Date, end = new Date()) {
  return Math.floor((end.getTime() - start.getTime()) / DAY);
}

export function fmtDate(date: Date | undefined) {
  return date ? date.toLocaleDateString(navigator.language) : "never";
}

export function fmtRange(start: Date, end: Date | undefined) {
  let res = fmtDate(start);
  if (end && end < maxDate) res += " - " + fmtDate(end);
  return res;
}

export function fmtSeconds(sec_num = 0) {
  let secs = Math.abs(sec_num);
  let hours = Math.floor(Math.abs(sec_num) / 3600);
  let minutes: number | string = Math.floor((secs % 3600) / 60);
  if (minutes < 10) {
    minutes = "0" + minutes;
  }
  return (sec_num < 0 ? "-" : "") + hours + ":" + minutes;
}

export function fmtNumber(number: number | undefined) {
  return (number || 0).toLocaleString(navigator.language);
}

export function myfetch(url: string, method?: any, data?: any) {
  let option: RequestInit | undefined;
  if (method) {
    option = {
      method: method, // *GET, POST, PUT, DELETE, etc.
      credentials: "include", // include, *same-origin, omit
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(data), // body data type must match "Content-Type" header
    };
  }
  return fetch(url, option).then(checkStatus);
}

export function checkStatus<T>(response: Response) {
  if (response.ok) {
    return response.json();
  }

  if (response.status === 401) {
    user.set(undefined);
    window.location.href = "/#/about";
  }

  return response.text().then((text) => {
    message.set({ active: true, status: response.statusText, message: text });
    return Promise.reject(text);
  });
}

export function handleError(e: Error) {
  message.update((m) => {
    if (m.active == false) {
      m.message = e.message;
      m.active = true;
    }
    return m;
  });
}

export const icons = new Map([
  [1, "flaticon-mountain-bike"],
  [301, "flaticon-run"],
  [302, "flaticon-snow"],
  [303, "flaticon-ski"],
]);

export async function getTypes() {
  return Promise.all([
    myfetch("/api/types/part").then((types) =>
      types.map((t: any) => new Type(t)).reduce(mapObject("id"), {}),
    ), // data[0]
    myfetch("/api/types/activity"), // data[1]
  ])
    .then((data: { 0: Type[]; 1: ActType[] }) => {
      types = data[1].reduce((acc, a) => {
        acc[a.gear_type].acts.push(a);
        return acc;
      }, data[0]);
    })
    .then(() => (category = writable(types[1])));
}

export async function initData() {
  let u = await myfetch("/api/user");
  if (u) {
    user.set(u);
  } else {
    return;
  }
  return Promise.all([myfetch("/api/user/summary").then(setSummary)]);
}

type Summary = {
  parts: Part[];
  attachments: Attachment[];
  activities: Activity[];
  usages: Usage[];
  services: Service[];
  plans: ServicePlan[];
};

export function setSummary(data: Summary) {
  usages.setMap(data.usages);
  parts.setMap(data.parts);
  attachments.setMap(data.attachments);
  activities.setMap(data.activities);
  services.setMap(data.services);
  plans.setMap(data.plans);
}

export function updateSummary(data: Summary) {
  parts.updateMap(data.parts);
  attachments.updateMap(data.attachments);
  activities.updateMap(data.activities);
  services.updateMap(data.services);
  plans.updateMap(data.plans);
  usages.updateMap(data.usages);
}

export let types: { [key: number]: Type };

export let category: Writable<Type>;
export const user = writable<User | undefined>(undefined);

export const state = writable({ show_all_spares: false });
export const message = writable({
  active: false,
  message: "No message",
  status: "",
});
