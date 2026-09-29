import type { Type } from "./types";
import type { Part } from "./part";
import type { PartNote } from "./partnote";
import type { Attachment } from "./attachment";
import type { Service } from "./service";
import type { ServicePlan } from "./serviceplan";
import type { Activity } from "./activity";
import type { Shop } from "./shop";

/**
 * The modal action registry: the type of the const registry exported by
 * Widgets/Actions.svelte. One entry per modal-opening operation; readers
 * call the registry directly.
 */
export type ModalType = {
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
