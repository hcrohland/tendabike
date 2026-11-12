<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import { Activity } from "../lib/activity";
  import Menu from "../Widgets/Menu.svelte";
  import ProviderMark from "../Widgets/ProviderMark.svelte";
  import { changeActivity } from "./ChangeActivity.svelte";
  import { providerOfActivityId, providerMeta } from "../lib/provider";
  import * as m from "../../paraglide/messages";

  let { row }: { row: Activity } = $props();

  let provider = $derived(providerOfActivityId(row.id));
</script>

<div class="flex justify-between">
  <a href={"/source/activities/" + row.id} target="_blank">
    {row.name}
    <span
      class="inline mx-2 align-middle"
      title={m.gearcard_view_on({ provider: providerMeta[provider].name })}
    >
      <ProviderMark {provider} size="sm" />
    </span>
  </a>
  <Menu>
    <DropdownItem onclick={() => changeActivity(row)}>
      {m.action_change()}
    </DropdownItem>
  </Menu>
</div>
