<script lang="ts">
  import type { Part } from "../lib/part";
  import { Service } from "../lib/service";
  import type { ServicePlan } from "../lib/serviceplan";
  import DeleteService from "./DeleteService.svelte";
  import ServiceModal from "./ServiceModal.svelte";
  import { m } from "../../paraglide/messages";

  let modal: { start: (s: Service) => void };
  let deleteService: { start: (s: Service) => void };
  let saveService = $state(saveNew);
  let title = $state("");

  export function change(s: Service) {
    saveService = saveUpdate;
    title = m.action_change();
    modal.start(s);
  }

  export function create(part: Part, plan?: ServicePlan) {
    saveService = saveNew;
    title = m.action_create();
    modal.start(
      new Service({ part_id: part.id, plans: plan ? [plan.id] : [] }),
    );
  }

  export function repeat(s: Service) {
    saveService = saveRepeat;
    title = m.action_repeat();
    modal.start(s);
  }

  export function del(s: Service) {
    deleteService.start(s);
  }

  async function saveUpdate(newservice: Service) {
    await newservice.update();
  }

  async function saveNew(newservice: Service) {
    await Service.create(
      newservice.part_id,
      newservice.time,
      newservice.name,
      newservice.notes,
      newservice.plans,
    );
  }

  async function saveRepeat(newservice: Service) {
    await newservice.repeat();
  }
</script>

<ServiceModal {saveService} bind:this={modal}>{title}</ServiceModal>
<DeleteService bind:this={deleteService} />
