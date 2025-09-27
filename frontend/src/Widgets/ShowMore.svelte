<script lang="ts">
  import { Button, Tooltip } from "flowbite-svelte";
  interface Props {
    show_more: boolean;
    title?: string | undefined;
    update?: (v: boolean) => void;
  }

  let {
    show_more = $bindable(false),
    title = undefined,
    update = (v) => {},
  }: Props = $props();

  function onclick() {
    show_more = !show_more;
    update(show_more);
  }
  let id = $props.id();

  let delay = 1000;
</script>

<Button
  {id}
  size="sm"
  color="alternative"
  class="border-0 p-0 m-0 me-3"
  {onclick}
>
  {show_more ? "-" : "+"}
</Button>
{#if title}
  <Tooltip placement="top" class=" text-xs p-1 rounded-xs">
    Show {title}
  </Tooltip>
{/if}
