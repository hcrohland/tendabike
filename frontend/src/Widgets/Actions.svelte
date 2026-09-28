<script context="module" lang="ts">
  export { getActions, setActions } from "../lib/actions.svelte";
</script>

<script lang="ts">
  import { setActions } from "../lib/actions.svelte";
  import DeletePlan, {
    start as deletePlan,
  } from "../ServicePlan/DeletePlan.svelte";
  import NewNote, { start as newNote } from "../Part/NewNote.svelte";
  import DeleteNote, { start as deleteNote } from "../Part/DeleteNote.svelte";
  import UpdatePlan from "../ServicePlan/UpdatePlan.svelte";
  import ServiceActions from "../Service/ServiceActions.svelte";
  import NewPlan from "../ServicePlan/NewPlan.svelte";
  import NewPart, { start as newPart } from "../Part/NewPart.svelte";
  import RecoverPart, {
    start as recoverPart,
  } from "../Part/RecoverPart.svelte";
  import DisposePart, {
    start as disposePart,
  } from "../Part/DisposePart.svelte";
  import InstallPart, {
    start as installPart,
  } from "../Attachment/InstallPart.svelte";
  import type { Part } from "../lib/part";
  import ReplacePart, {
    start as replacePart,
  } from "../Attachment/ReplacePart.svelte";
  import type { ServicePlan } from "../lib/serviceplan";
  import ChangePart, { start as changePart } from "../Part/ChangePart.svelte";
  import DeleteAttachment, {
    start as deleteAttachment,
  } from "../Attachment/DeleteAttachment.svelte";
  import AttachPart, {
    start as attachPart,
  } from "../Attachment/AttachPart.svelte";
  import ChangeActivity, {
    start as changeActivity,
  } from "../Activity/ChangeActivity.svelte";
  import DeletePart, { start as deletePart } from "../Part/DeletePart.svelte";
  import ShopModal, { start as shopModal } from "../Shop/ShopModal.svelte";
  import DeleteShop, { start as deleteShop } from "../Shop/DeleteShop.svelte";
  import SubscriptionRequestModal, {
    start as requestSubscription,
  } from "../Shop/SubscriptionRequestModal.svelte";
  import type { Service } from "../lib/service";

  $: setActions({
    newPart,
    newNote,
    deleteNote,
    installPart,
    changePart,
    deletePart,
    disposePart,
    recoverPart,
    attachPart,
    replacePart,
    newService: serviceActions?.create,
    redoService: serviceActions?.repeat,
    changeService: serviceActions?.change,
    deleteService: serviceActions?.del,
    newPlan: newPlan?.start,
    updatePlan: updatePlan?.start,
    deletePlan,
    deleteAttachment,
    changeActivity,
    createShop: () => shopModal(),
    editShop: shopModal,
    deleteShop,
    requestSubscription,
  });

  let serviceActions: {
    create: (part: Part, plan?: ServicePlan) => void;
    repeat: (s: Service) => void;
    del: (s: Service) => void;
    change: (s: Service) => void;
  };
  let newPlan: { start: (p: Part) => void };
  let updatePlan: { start: (p: ServicePlan) => void };
</script>

<NewPart />
<NewNote />
<DeleteNote />
<ChangePart />
<DeletePart />
<ServiceActions bind:this={serviceActions} />
<NewPlan bind:this={newPlan} />
<UpdatePlan bind:this={updatePlan} />
<DeletePlan />
<RecoverPart />
<DisposePart />
<InstallPart />
<ReplacePart />
<AttachPart />
<DeleteAttachment />
<ChangeActivity />
<ShopModal />
<DeleteShop />
<SubscriptionRequestModal />
