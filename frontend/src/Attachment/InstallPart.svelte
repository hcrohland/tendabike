<script lang="ts">
  import { InputAddon, ButtonGroup } from "flowbite-svelte";
  import { Type } from "../lib/types";
  import { user } from "../lib/store";
  import NewForm from "../Part/PartForm.svelte";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import { filterValues } from "../lib/mapable";
  import { Part } from "../lib/part";
  import { attachments } from "../lib/attachment";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import Modal from "../Widgets/Modal.svelte";

  let part = $state<any>();
  let gear = $state(new Part({}));
  let type = $state<Type>();
  let hook = $state<number>();
  let open = $state(false);
  let single = $state(true);

  export const start = (g: Part) => {
    gear = g;
    part = {
      ...new Part({
        owner: $user && $user.id,
      }),
    };
    type = undefined;
    open = true;
  };

  async function attachPart(part: Part | void) {
    if (!part) return;
    await part.attach(part.purchase, !single, gear!.id!, hook!);
  }

  async function onaction() {
    await new Part(part).create().then(attachPart);
    open = false;
  }

  function guessDate(g: Part, t: Type, hook: number | undefined) {
    if (!t) return new Date();
    let last = filterValues(
      $attachments,
      (a) => a.gear == g.id && a.what == t.id && a.hook == hook,
    );
    if (last.length) {
      // It is a replacement
      return new Date();
    } else {
      // It is the first part of that type
      return g.purchase;
    }
  }

  part = new Part({
    owner: $user && $user.id,
    purchase: new Date(),
    last_used: new Date(),
  });

  const setType = (t: Type, h: number | undefined) => {
    part.what = t.id;
    part.hook = h;
    type = t;
    hook = h;
    part.purchase = guessDate(gear, t, h);
  };
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    <ButtonGroup class="col-md-12">
      <InputAddon>New</InputAddon>
      <TypeForm
        onChange={setType}
        classes={{ select: "rounded-none h-full" }}
      />
      <InputAddon>of {gear.name}</InputAddon>
    </ButtonGroup>
  {/snippet}

  <NewForm {type} bind:part mindate={gear.purchase} />
  {#if type?.is_hook()}
    <Switch bind:checked={single}>Keep all attached parts</Switch>
  {/if}

  {#snippet footer()}
    <Buttons bind:open label="Install" />
  {/snippet}
</Modal>
