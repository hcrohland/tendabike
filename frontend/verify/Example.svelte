<script module lang="ts">
  import { Part, parts } from "../src/lib/part";
  import { Service, services } from "../src/lib/service";
  import { ServicePlan, plans } from "../src/lib/serviceplan";
  import { Usage, usages } from "../src/lib/usage";

  // Seed the state objects the migrated components read, so the harness
  // renders real Part/Service/ServicePlan components without the full stack.
  parts.setMap([
    new Part({
      id: 5,
      owner: 1,
      what: 10,
      name: "Wheel",
      vendor: "Rider",
      model: "700c",
      purchase: "2023-01-01T00:00:00Z",
      last_used: "2024-01-01T00:00:00Z",
      usage: "u1",
    }),
  ]);
  usages.setMap([
    new Usage({
      id: "u1",
      count: 10,
      climb: 100,
      descend: 80,
      distance: 50000,
      time: 10800,
      duration: 10800,
      energy: 300,
    }),
  ]);
  plans.setMap([
    new ServicePlan({
      id: "P1",
      part: 5,
      what: 10,
      hook: null,
      name: "Wheel Service",
      km: "100",
    }),
  ]);
  services.setMap([
    new Service({
      id: "S1",
      part_id: 5,
      time: "2024-01-01T00:00:00Z",
      redone: "2023-06-01T00:00:00Z",
      name: "Annual Service",
      notes: "",
      usage: "u1",
      successor: null,
      plans: ["P1"],
    }),
    new Service({
      id: "S0",
      part_id: 5,
      time: "2023-06-01T00:00:00Z",
      redone: "2023-01-01T00:00:00Z",
      name: "Earlier Service",
      notes: "",
      usage: "u1",
      successor: "S1",
      plans: ["P1"],
    }),
  ]);
</script>

<script lang="ts">
  import { Button } from "flowbite-svelte";
  import Card from "../src/Widgets/Card.svelte";
  import Chip from "../src/Widgets/Chip.svelte";
  import PlanBlock from "../src/ServicePlan/PlanBlock.svelte";
  import ServiceHist from "../src/Service/ServiceHist.svelte";
  import { stateValues } from "../src/lib/mapable.svelte";
</script>

<!--
  The harness example. Renders the migrated neutral components — the wrapper
  card/chip/fill scenarios, plus the real migrated Part/Service/ServicePlan
  components (a plan block: secondary card + nested primary row + matching
  button; a service history card: secondary card + nested primary history
  block) — with class anchors the real-browser check reads computed styles
  from.
-->
<div class="p-6 max-w-md space-y-6">
  <section data-testid="scenario-card">
    <p class="text-sm font-semibold mb-2">card (secondary) + matching chip</p>
    <Card color="secondary">
      <div class="p-2">
        <Chip value="5" label="Rides" color="secondary" />
      </div>
    </Card>
  </section>

  <section data-testid="scenario-nested">
    <p class="text-sm font-semibold mb-2">nested card flip</p>
    <Card color="secondary">
      <p>outer card (secondary surface)</p>
      <Card color="primary">
        <p>inner card (primary surface)</p>
      </Card>
    </Card>
  </section>

  <section data-testid="scenario-fills">
    <p class="text-sm font-semibold mb-2">primary vs secondary fill</p>
    <div class="flex flex-wrap gap-2">
      <Button color="primary" data-testid="fill-primary">primary</Button>
      <Button color="secondary" data-testid="fill-secondary">secondary</Button>
    </div>
  </section>

  <section data-testid="scenario-planblock">
    <p class="text-sm font-semibold mb-2">
      migrated PlanBlock (secondary card, nested primary row)
    </p>
    {#each stateValues(plans) as plan (plan.id)}
      <PlanBlock {plan} />
    {/each}
  </section>

  <section data-testid="scenario-servicehist">
    <p class="text-sm font-semibold mb-2">
      migrated ServiceHist (secondary card)
    </p>
    {#each stateValues(services) as service (service.id)}
      {#if service.successor == null}
        <ServiceHist {service} />
      {/if}
    {/each}
  </section>
</div>
