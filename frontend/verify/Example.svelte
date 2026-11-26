<script lang="ts">
  import { Button } from "flowbite-svelte";
  import Card from "../src/Widgets/Card.svelte";
  import Chip from "../src/Widgets/Chip.svelte";
</script>

<!--
  The harness example. Renders the migrated neutral components — a card and a
  nested card (surfaces), a chip matching its card (a fill), and primary vs
  secondary fill buttons — with class anchors the real-browser check reads
  computed styles from.
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
</div>
