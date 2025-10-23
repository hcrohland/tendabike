<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import { Activity } from "../lib/activity";
  import Menu from "../Widgets/Menu.svelte";
  import { actions } from "../Widgets/Actions.svelte";

  export let row: Activity;
</script>

<div class="flex justify-between">
  <a href={"/strava/activities/" + row.id} target="_blank">
    {row.name}
    <img
      src="strava_grey.png"
      alt="View on Strava"
      title="View on Strava"
      class="inline mx-2"
    />
  </a>
  <Menu>
    <DropdownItem onclick={() => $actions.changeActivity(row)}>
      Change
    </DropdownItem>
  </Menu>
</div>
