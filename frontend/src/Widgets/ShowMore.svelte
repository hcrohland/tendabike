<script lang="ts">
  import { Button, Tooltip } from "@sveltestrap/sveltestrap";
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

<Button {id} color="light" class="m-0 p-1" {onclick}>
  {#if show_more}
    −
  {:else}
    +
    {#if title}
      <Tooltip placement="left" {delay} target={id}>Show {title}</Tooltip>
    {/if}
  {/if}
</Button>
