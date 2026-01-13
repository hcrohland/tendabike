import { writable } from "svelte/store";
import { Service, services } from "./service";
import { activities, Activity } from "./activity";
import { Usage, usages } from "./usage";
import { parts, type Part } from "./part";
import { Attachment, attachments } from "./attachment";
import { plans, type ServicePlan } from "./serviceplan";
import { Shop, shops } from "./shop";
import { myfetch } from "./store";
import { mapable } from "./mapable";

export async function initData() {
  let u = await myfetch("/api/user");
  if (u) {
    user.set(u);
  } else {
    return;
  }
  return refresh();
}

export async function refresh(shop?: number) {
  let query = shop ? "?shop=" + shop : "";
  await myfetch("/api/user/summary" + query).then(setSummary);
}

export type UserPublic = {
  id: number;
  firstname: string;
  name: string;
  avatar: string | undefined;
};

export type OnboardingStatus =
  | "pending"
  | "completed"
  | "initial_sync_postponed";

export type User = {
  id: number;
  firstname: string;
  name: string;
  avatar: string | undefined;
  is_admin: boolean;
  onboarding_status: OnboardingStatus;
};

type Summary = {
  parts: Part[];
  attachments: Attachment[];
  activities: Activity[];
  usages: Usage[];
  services: Service[];
  plans: ServicePlan[];
  shops: Shop[];
  users: UserPublic[];
};

export function setSummary(data: Summary) {
  usages.setMap(data.usages);
  parts.setMap(data.parts);
  attachments.setMap(data.attachments);
  activities.setMap(data.activities);
  services.setMap(data.services);
  plans.setMap(data.plans);
  shops.setMap(data.shops);
  users.setMap(data.users);
}

export function updateSummary(data?: Summary) {
  if (!data) {
    refresh();
    return;
  }
  parts.updateMap(data.parts);
  attachments.updateMap(data.attachments);
  activities.updateMap(data.activities);
  services.updateMap(data.services);
  plans.updateMap(data.plans);
  usages.updateMap(data.usages);
  shops.updateMap(data.shops);
}

export const user = writable<User | undefined>(undefined);
export const users = mapable<UserPublic>("id");

export const state = writable({ show_all_spares: false });
