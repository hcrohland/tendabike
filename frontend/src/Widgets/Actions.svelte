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
  import UpdatePlan, {
    start as updatePlan,
  } from "../ServicePlan/UpdatePlan.svelte";
  import ServiceActions, {
    create as newService,
    repeat as redoService,
    change as changeService,
    del as deleteService,
  } from "../Service/ServiceActions.svelte";
  import NewPlan, { start as newPlan } from "../ServicePlan/NewPlan.svelte";
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
  import ReplacePart, {
    start as replacePart,
  } from "../Attachment/ReplacePart.svelte";
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
    newService,
    redoService,
    changeService,
    deleteService,
    newPlan,
    updatePlan,
    deletePlan,
    deleteAttachment,
    changeActivity,
    createShop: () => shopModal(),
    editShop: shopModal,
    deleteShop,
    requestSubscription,
  });
</script>

<NewPart />
<NewNote />
<DeleteNote />
<ChangePart />
<DeletePart />
<ServiceActions />
<NewPlan />
<UpdatePlan />
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
