<script context="module" lang="ts">
  import { writable } from "svelte/store";

  type ModalType = {
    newPart: (t: Type) => void;
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
  };

  export let actions = writable<ModalType>();
</script>

<script lang="ts">
  import DeletePlan from "../ServicePlan/DeletePlan.svelte";
  import UpdatePlan from "../ServicePlan/UpdatePlan.svelte";
  import ServiceActions from "../Service/ServiceActions.svelte";
  import NewPlan from "../ServicePlan/NewPlan.svelte";
  import NewPart from "../Part/NewPart.svelte";
  import RecoverPart from "../Part/RecoverPart.svelte";
  import DisposePart from "../Part/DisposePart.svelte";
  import InstallPart from "../Attachment/InstallPart.svelte";
  import type { Part } from "../lib/part";
  import ReplacePart from "../Attachment/ReplacePart.svelte";
  import type { Attachment } from "../lib/attachment";
  import type { ServicePlan } from "../lib/serviceplan";
  import type { Type } from "../lib/types";
  import type { Service } from "../lib/service";
  import ChangePart from "../Part/ChangePart.svelte";
  import DeleteAttachment from "../Attachment/DeleteAttachment.svelte";
  import AttachPart from "../Attachment/AttachPart.svelte";
  import ChangeActivity from "../Activity/ChangeActivity.svelte";
  import type { Activity } from "../lib/activity";
  import DeletePart from "../Part/DeletePart.svelte";

  $: actions.set({
    newPart: newPart?.start,
    installPart: installPart?.start,
    changePart: changePart?.start,
    deletePart: deletePart?.start,
    disposePart: disposePart?.start,
    recoverPart: recoverPart?.start,
    attachPart: attachPart?.start,
    replacePart: replacePart?.start,
    newService: serviceActions?.create,
    redoService: serviceActions?.repeat,
    changeService: serviceActions?.change,
    deleteService: serviceActions?.del,
    newPlan: newPlan?.start,
    updatePlan: updatePlan?.start,
    deletePlan: deletePlan?.start,
    deleteAttachment: deleteAttachment?.start,
    changeActivity: changeActivity?.start,
  });

  let newPart: { start: (t: Type) => void };
  let installPart: { start: (p: Part) => void };
  let changePart: { start: (p: Part) => void };
  let deletePart: { start: (p: Part) => void };
  let disposePart: { start: (p: Part, a?: Attachment) => void };
  let replacePart: { start: (p: Attachment) => void };
  let recoverPart: { start: (p: Part) => void };
  let attachPart: { start: (p: Part) => void };
  let deleteAttachment: { start: (a: Attachment) => void };
  let serviceActions: {
    create: (part: Part, plan?: ServicePlan) => void;
    repeat: (s: Service) => void;
    del: (s: Service) => void;
    change: (s: Service) => void;
  };
  let newPlan: { start: (p: Part) => void };
  let updatePlan: { start: (p: ServicePlan) => void };
  let deletePlan: { start: (p: ServicePlan) => void };
  let changeActivity: { start: (a: Activity) => void };
</script>

<NewPart bind:this={newPart} />
<ChangePart bind:this={changePart} />
<DeletePart bind:this={deletePart} />
<ServiceActions bind:this={serviceActions} />
<NewPlan bind:this={newPlan} />
<UpdatePlan bind:this={updatePlan} />
<DeletePlan bind:this={deletePlan} />
<RecoverPart bind:this={recoverPart} />
<DisposePart bind:this={disposePart} />
<InstallPart bind:this={installPart} />
<ReplacePart bind:this={replacePart} />
<AttachPart bind:this={attachPart} />
<DeleteAttachment bind:this={deleteAttachment} />
<ChangeActivity bind:this={changeActivity} />
