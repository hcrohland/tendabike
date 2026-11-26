<script lang="ts">
  import { Badge, Button } from "flowbite-svelte";
  import Card from "../src/Widgets/Card.svelte";
  import Tone from "../src/Widgets/Tone.svelte";
</script>

<!--
  The harness example. Swap in the component under test, and keep a
  data-testid anchor on whatever you will assert. Import real app
  components across the boundary, as here.
-->
<div class="p-6 max-w-md space-y-6">
  <section data-testid="scenario-default">
    <p class="text-sm font-semibold mb-2">card + tone content</p>
    <Card>
      <Tone>content on the contrasting tone</Tone>
    </Card>
  </section>

  <section data-testid="scenario-nested">
    <p class="text-sm font-semibold mb-2">nested card flip</p>
    <Card>
      <p>outer card (darker tone)</p>
      <Card>
        <p>inner card (flipped to the light tone)</p>
      </Card>
    </Card>
  </section>

  <section data-testid="scenario-status">
    <p class="text-sm font-semibold mb-2">status + neutral components</p>
    <div class="flex flex-wrap gap-2">
      <Badge color="red">danger</Badge>
      <Badge color="yellow">warning</Badge>
      <Badge color="blue">info</Badge>
      <Button color="light">neutral</Button>
    </div>
  </section>
</div>
