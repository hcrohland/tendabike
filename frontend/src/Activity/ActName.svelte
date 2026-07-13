<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import { Activity } from "../lib/activity";
  import Menu from "../Widgets/Menu.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import * as m from "../../paraglide/messages";

  export let row: Activity;
</script>

<div class="flex justify-between">
  <a href={"/strava/activities/" + row.id} target="_blank">
    {row.name}
    <img
      src="strava_grey.png"
      alt={m.gearcard_view_on_strava()}
      title={m.gearcard_view_on_strava()}
      class="inline mx-2"
    />
  </a>
  <Menu>
    <DropdownItem onclick={() => $actions.changeActivity(row)}>
      {m.action_change()}
    </DropdownItem>
  </Menu>
</div>
