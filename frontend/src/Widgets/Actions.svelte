<script module lang="ts">
  import DeletePlan, { deletePlan } from "../ServicePlan/DeletePlan.svelte";
  import NewNote, { newNote } from "../Part/NewNote.svelte";
  import DeleteNote, { deleteNote } from "../Part/DeleteNote.svelte";
  import PlanModal, {
    newPlan,
    updatePlan,
  } from "../ServicePlan/PlanModal.svelte";
  import ServiceActions, {
    newService,
    redoService,
    changeService,
    deleteService,
  } from "../Service/ServiceActions.svelte";
  import NewPart, { newPart } from "../Part/NewPart.svelte";
  import RecoverPart, { recoverPart } from "../Part/RecoverPart.svelte";
  import DisposePart, { disposePart } from "../Part/DisposePart.svelte";
  import InstallPart, { installPart } from "../Attachment/InstallPart.svelte";
  import ReplacePart, { replacePart } from "../Attachment/ReplacePart.svelte";
  import ChangePart, { changePart } from "../Part/ChangePart.svelte";
  import DeleteAttachment, {
    deleteAttachment,
  } from "../Attachment/DeleteAttachment.svelte";
  import AttachPart, { attachPart } from "../Attachment/AttachPart.svelte";
  import ChangeActivity, {
    changeActivity,
  } from "../Activity/ChangeActivity.svelte";
  import DeletePart, { deletePart } from "../Part/DeletePart.svelte";
  import ShopModal, { createShop, editShop } from "../Shop/ShopModal.svelte";
  import DeleteShop, { deleteShop } from "../Shop/DeleteShop.svelte";
  import SubscriptionRequestModal, {
    requestSubscription,
  } from "../Shop/SubscriptionRequestModal.svelte";

  /**
   * The modal action registry, as a const: it never changes after startup
   * and nothing reads it reactively, so it holds no reactive machinery.
   * History: it was a svelte/store writable, converted to a Svelte 5 state
   * object by the store-to-state migration (nullable until this widget
   * mounted, replaced wholesale once at mount, read via `getActions` from
   * the dedicated lib/actions.svelte state module, called through
   * `getActions()!` at every reader), and is now a plain const referencing
   * each modal's module-scoped start operation directly.
   */
  export const actions = {
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
    createShop,
    editShop,
    deleteShop,
    requestSubscription,
  };
</script>

<NewPart />
<NewNote />
<DeleteNote />
<ChangePart />
<DeletePart />
<ServiceActions />
<PlanModal />
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
