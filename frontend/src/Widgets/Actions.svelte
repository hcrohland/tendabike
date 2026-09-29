<script module lang="ts">
  // Mounts every modal once, globally: callers import each modal's start
  // function from the modal module directly, and the mounted instance
  // below reacts to it.
  import DeletePlan from "../ServicePlan/DeletePlan.svelte";
  import NewNote from "../Part/NewNote.svelte";
  import DeleteNote from "../Part/DeleteNote.svelte";
  import PlanModal from "../ServicePlan/PlanModal.svelte";
  import ServiceActions from "../Service/ServiceActions.svelte";
  import NewPart from "../Part/NewPart.svelte";
  import RecoverPart from "../Part/RecoverPart.svelte";
  import DisposePart from "../Part/DisposePart.svelte";
  import InstallPart from "../Attachment/InstallPart.svelte";
  import ReplacePart from "../Attachment/ReplacePart.svelte";
  import ChangePart from "../Part/ChangePart.svelte";
  import DeleteAttachment from "../Attachment/DeleteAttachment.svelte";
  import AttachPart from "../Attachment/AttachPart.svelte";
  import ChangeActivity from "../Activity/ChangeActivity.svelte";
  import DeletePart from "../Part/DeletePart.svelte";
  import ShopModal from "../Shop/ShopModal.svelte";
  import DeleteShop from "../Shop/DeleteShop.svelte";
  import SubscriptionRequestModal from "../Shop/SubscriptionRequestModal.svelte";
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
