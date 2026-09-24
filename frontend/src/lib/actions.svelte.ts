import type { Type } from "./types";
import type { Part } from "./part";
import type { PartNote } from "./partnote";
import type { Attachment } from "./attachment";
import type { Service } from "./service";
import type { ServicePlan } from "./serviceplan";
import type { Activity } from "./activity";
import type { Shop } from "./shop";

type ModalType = {
  newPart: (t: Type) => void;
  newNote: (p: Part, note?: PartNote) => void;
  deleteNote: (n: PartNote) => void;
  installPart: (p: Part) => void;
  changePart: (p: Part) => void;
  deletePart: (p: Part) => void;
  disposePart: (p: Part, a?: Attachment) => void;
  recoverPart: (p: Part) => void;
  replacePart: (p: Attachment) => void;
  attachPart: (p: Part) => void;
  newService: (part: Part, plan?: ServicePlan) => void;
  newPlan: (p: Part) => void;
  changeService: (s: Service) => void;
  redoService: (s: Service) => void;
  deleteService: (s: Service) => void;
  updatePlan: (p: ServicePlan) => void;
  deletePlan: (p: ServicePlan) => void;
  deleteAttachment: (a: Attachment) => void;
  changeActivity: (a: Activity) => void;
  createShop: () => void;
  editShop: (g: Shop) => void;
  deleteShop: (g: Shop) => void;
  requestSubscription: (g: Shop) => void;
};

/**
 * The modal action registry, as a Svelte 5 state object (the store-to-state
 * migration of the former svelte/store writable in Widgets/Actions.svelte).
 * The value is nullable (unset until Actions.svelte mounts its modals) and
 * replaced wholesale, which a `.svelte.ts` module may not export directly
 * (Svelte's `state_invalid_export` rule), so the state lives in the module
 * and is read via `getActions` and replaced via `setActions`. Reads register
 * dependencies at the enclosing reactive call site; readers use
 * `getActions()!` where they formerly used `$actions`.
 */
let actions = $state<ModalType | undefined>(undefined);

export function getActions(): ModalType | undefined {
  return actions;
}

/** Replace the modal actions; `undefined` clears them. */
export function setActions(value: ModalType | undefined) {
  actions = value;
}
