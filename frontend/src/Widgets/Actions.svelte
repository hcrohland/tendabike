<script context="module" lang="ts">
  import { writable } from "svelte/store";

  type ModalType = {
    newPart: (t: Type) => void;
    installPart: (p: Part) => void;
    changePart: (p: Part) => void;
    recoverPart: (p: Part) => void;
    replacePart: (p: Attachment) => void;
    attachPart: (p: Part) => void;
    newService: (part: Part | null, plans?: string[]) => void;
    newPlan: (p?: Part) => void;
    redoService: (s: Service | undefined) => void;
    updatePlan: (p: ServicePlan) => void;
    deletePlan: (p: ServicePlan) => void;
  };

  export let actions = writable<ModalType>();
</script>

<script lang="ts">
  import AttachPart from "../Attachment/AttachPart.svelte";
  import InstallPart from "../Attachment/InstallPart.svelte";
  import ReplacePart from "../Attachment/ReplacePart.svelte";
  import ChangePart from "../Part/ChangePart.svelte";
  import NewPart from "../Part/NewPart.svelte";
  import RecoverPart from "../Part/RecoverPart.svelte";
  import NewService from "../Service/NewService.svelte";
  import RedoService from "../Service/RedoService.svelte";
  import DeletePlan from "../ServicePlan/DeletePlan.svelte";
  import NewPlan from "../ServicePlan/NewPlan.svelte";
  import UpdatePlan from "../ServicePlan/UpdatePlan.svelte";
  import type { Attachment } from "../lib/attachment";
  import type { Part } from "../lib/part";
  import type { Service } from "../lib/service";
  import type { ServicePlan } from "../lib/serviceplan";
  import type { Type } from "../lib/types";

  $: actions.set({
    newPart,
    installPart,
    changePart,
    recoverPart,
    attachPart,
    replacePart,
    newService,
    redoService,
    newPlan,
    updatePlan,
    deletePlan,
  });

  let newPart: (t: Type) => void;
  let installPart: (p: Part) => void;
  let changePart: (p: Part) => void;
  let replacePart: (p: Attachment) => void;
  let recoverPart: (p: Part) => void;
  let attachPart: (p: Part) => void;

  let newService: (part: Part | null, plans?: string[]) => void;
  let redoService: (s: Service | undefined) => void;

  let newPlan: (p?: Part) => void;
  let updatePlan: (p: ServicePlan) => void;
  let deletePlan: (p: ServicePlan) => void;
</script>

<NewPart bind:newPart />
<AttachPart bind:attachPart />
<ChangePart bind:changePart />
<InstallPart bind:installPart />
<RecoverPart bind:recoverPart />
<ReplacePart bind:replacePart />

<NewService bind:newService />
<RedoService bind:redoService />

<NewPlan bind:newPlan />
<UpdatePlan bind:updatePlan />
<DeletePlan bind:deletePlan />
