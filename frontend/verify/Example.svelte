<script module lang="ts">
  import { Part, parts } from "../src/lib/part";
  import { Service, services } from "../src/lib/service";
  import { ServicePlan, plans } from "../src/lib/serviceplan";
  import { Usage, usages } from "../src/lib/usage";
  import { Type } from "../src/lib/types.svelte";
  import { Shop, shops } from "../src/lib/shop.svelte";

  // The harness has no backend: stub the app's /api/* fetches with empty
  // JSON so components that load data (shop parts, the offline banner's
  // providers) settle to a renderable state.
  const realFetch = window.fetch;
  window.fetch = (async (input: any, init?: any) => {
    if (String(input).includes("/api/")) {
      return new Response("[]", {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }
    return realFetch(input, init);
  }) as typeof fetch;

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
    // A disposed spare: the spare-type selector renders it on the secondary
    // surface with the secondary chip fill (behind the show-more toggle).
    new Part({
      id: 6,
      owner: 1,
      what: 10,
      name: "Old Wheel",
      vendor: "Rider",
      model: "700c",
      purchase: "2022-01-01T00:00:00Z",
      last_used: "2023-01-01T00:00:00Z",
      usage: "u1",
      disposed_at: "2024-06-01T00:00:00Z",
    }),
  ]);
  // The spare-type selector reads its parts through the `parts` map (seeded
  // above); the type instance itself needs no module state.
  const spareType = new Type({
    id: 10,
    name: "Wheel",
    main: 1,
    hooks: [1],
    order: 1,
  });
  shops[10] = new Shop({
    id: 10,
    owner: 1,
    name: "Velo Shop",
    description: "Local shop",
    auto_approve: true,
    created_at: "2023-01-01T00:00:00Z",
  });
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
  import ShopCard from "../src/Shop/ShopCard.svelte";
  import SpareType from "../src/Spares/SpareType.svelte";
  import AdminCard from "../src/Admin/AdminCard.svelte";
  import OfflineBanner from "../src/OfflineBanner.svelte";
  import { stateValues } from "../src/lib/mapable.svelte";
  import { loadProviders } from "../src/lib/provider.svelte";
  import type { AdminRow } from "../src/lib/user.svelte";

  const shop = shops[10] as Shop;
  const users = {
    1: { id: 1, firstname: "Max", name: "Mustermann", avatar: undefined },
  };
  const adminRow: AdminRow = {
    user: {
      id: 3,
      firstname: "Max",
      name: "Mustermann",
      avatar: undefined,
      is_admin: false,
      onboarding_status: "completed",
      provider: "strava",
    },
    parts: 2,
    activities: 7,
    events: 11,
    disabled: false,
    provider: "strava",
  };

  // The offline banner shows once the (stubbed) providers fetch settles
  // with an empty enabled set.
  $effect(() => {
    loadProviders();
  });
</script>

<!--
  The harness example. Renders the migrated neutral components — the wrapper
  card/chip/fill scenarios, plus the real migrated Part/Service/ServicePlan
  components (a plan block: secondary card + nested primary row + matching
  button; a service history card: secondary card + nested primary history
  block), the migrated Shop/Spares/Admin/top-level call sites (a shop card,
  the spare-type selector, an admin card, the offline banner) — with class
  anchors the real-browser check reads computed styles from.
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

  <section data-testid="scenario-shopcard">
    <p class="text-sm font-semibold mb-2">
      migrated ShopCard (secondary card, secondary chip fill)
    </p>
    <ShopCard {shop} {users} isOwner={true} />
  </section>

  <section data-testid="scenario-sparetype">
    <p class="text-sm font-semibold mb-2">
      migrated SpareType (secondary block, primary part card, primary chip fill)
    </p>
    <SpareType type={spareType} update={() => {}} attachee={0} />
  </section>

  <section data-testid="scenario-admincard">
    <p class="text-sm font-semibold mb-2">
      migrated AdminCard (secondary card, secondary chip fill)
    </p>
    <AdminCard
      row={adminRow}
      refresh={() => {}}
      createSync={() => {}}
      deleteUser={() => {}}
    />
  </section>

  <section data-testid="scenario-offline">
    <p class="text-sm font-semibold mb-2">
      migrated OfflineBanner (secondary surface)
    </p>
    <OfflineBanner />
  </section>
</div>
