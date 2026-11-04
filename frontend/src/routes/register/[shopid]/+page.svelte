<script lang="ts">
  import { page } from "$app/state";
  import Register from "../../../Shop/Register.svelte";
</script>

<Register params={{ shopid: Number(page.params.shopid) }} />
