<script lang="ts">
  import { page } from "$app/state";
  import Gear from "../../../Part/Gear.svelte";
</script>

<Gear params={{ id: Number(page.params.id) }} />
