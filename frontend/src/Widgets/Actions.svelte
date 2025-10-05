<script context="module" lang="ts">
  import { writable } from "svelte/store";

  // type ModalType = {
  //   newPart: (t: Type) => void;
  //   installPart: (p: Part) => void;
  //   changePart: (p: Part) => void;
  //   disposePart: (p: Part, a?: Attachment) => void;
  //   recoverPart: (p: Part) => void;
  //   replacePart: (p: Attachment) => void;
  //   attachPart: (p: Part) => void;
  //   newService: (part: Part | null, plans?: string[]) => void;
  //   newPlan: (p?: Part) => void;
  //   redoService: (s: Service | undefined) => void;
  //   updatePlan: (p: ServicePlan) => void;
  //   deletePlan: (p: ServicePlan) => void;
  //   deleteAttachment: (a: Attachment) => void;
  // };

  export let actions = writable<any>();
</script>

<script lang="ts">
  // import AttachPart from "../Attachment/AttachPart.svelte";
  // import InstallPart from "../Attachment/InstallPart.svelte";
  // import ReplacePart from "../Attachment/ReplacePart.svelte";
  // import ChangePart from "../Part/ChangePart.svelte";
  // import NewPart from "../Part/NewPart.svelte";
  // import RecoverPart from "../Part/RecoverPart.svelte";
  import DeletePlan from "../ServicePlan/DeletePlan.svelte";
  import UpdatePlan from "../ServicePlan/UpdatePlan.svelte";
  // import type { Attachment } from "../lib/attachment";
  // import type { Part } from "../lib/part";
  // import type { Service } from "../lib/service";
  // import type { ServicePlan } from "../lib/serviceplan";
  // import type { Type } from "../lib/types";
  // import DisposePart from "../Part/DisposePart.svelte";
  // import DeleteAttachment from "../Attachment/DeleteAttachment.svelte";
  import ServiceActions from "../Service/ServiceActions.svelte";
  import NewPlan from "../ServicePlan/NewPlan.svelte";
  import DeleteService from "../Service/DeleteService.svelte";

  $: actions.set({
    // newPart,
    // installPart,
    // changePart,
    // disposePart,
    // recoverPart,
    // attachPart,
    // replacePart,
    newService: serviceActions?.create,
    redoService: serviceActions?.repeat,
    changeService: serviceActions?.change,
    deleteService: serviceActions?.del,
    newPlan: newPlan?.start,
    updatePlan: updatePlan?.start,
    deletePlan: deletePlan?.start,
    // deleteAttachment,
  });

  let serviceActions: any, newPlan: any, updatePlan: any, deletePlan: any;
  // let newPart: (t: Type) => void;
  // let installPart: (p: Part) => void;
  // let changePart: (p: Part) => void;
  // let disposePart: (p: Part, a?: Attachment) => void;
  // let replacePart: (p: Attachment) => void;
  // let recoverPart: (p: Part) => void;
  // let attachPart: (p: Part) => void;

  // let newService: (part: Part | null, plans?: string[]) => void;
  // let redoService: (s: Service | undefined) => void;

  // let newPlan: (p?: Part) => void;
  // let updatePlan: (p: ServicePlan) => void;
  // let deletePlan: (p: ServicePlan) => void;
  // let deleteAttachment: (a: Attachment) => void;
</script>

<ServiceActions bind:this={serviceActions} />
<NewPlan bind:this={newPlan} />
<UpdatePlan bind:this={updatePlan} />
<DeletePlan bind:this={deletePlan} />
