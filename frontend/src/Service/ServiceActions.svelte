<script module lang="ts">
  import type { Part } from "../lib/part";
  import { Service } from "../lib/service";
  import type { ServicePlan } from "../lib/serviceplan";
  import { start as startService } from "./ServiceModal.svelte";
  import { start as startDeleteService } from "./DeleteService.svelte";
  import { m } from "../../paraglide/messages";

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

  const config = $state<{
    saveService: (p: Service) => void;
    title: string;
  }>({
    saveService: saveNew,
    title: "",
  });

  export function changeService(s: Service) {
    config.saveService = saveUpdate;
    config.title = m.action_change();
    startService(s);
  }

  export function newService(part: Part, plan?: ServicePlan) {
    config.saveService = saveNew;
    config.title = m.servicemodal_new();
    startService(
      new Service({ part_id: part.id, plans: plan ? [plan.id] : [] }),
    );
  }

  export function redoService(s: Service) {
    config.saveService = saveRepeat;
    config.title = m.action_repeat();
    startService(s);
  }

  export function deleteService(s: Service) {
    startDeleteService(s);
  }
</script>

<script lang="ts">
  import DeleteService from "./DeleteService.svelte";
  import ServiceModal from "./ServiceModal.svelte";
</script>

<ServiceModal saveService={config.saveService}>{config.title}</ServiceModal>
<DeleteService />
