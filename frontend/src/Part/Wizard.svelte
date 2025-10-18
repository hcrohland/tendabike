<script lang="ts">
  import { Input, ButtonGroup, Table, Button } from "flowbite-svelte";
  import { types, Type } from "../lib/types";
  import { Attachment } from "../lib/attachment";
  import { type Map, filterValues } from "../lib/mapable";
  import Switch from "../Widgets/Switch.svelte";
  import { Part } from "../lib/part";

  export let gear: Part;
  export let attachees: Attachment[];

  type Group = {
    group: string;
    enabled: boolean;
    types: Type[];
    vendor: string;
    model: string;
  };

  function groupBy(xs: Type[]) {
    return xs.reduce((rv: Map<Group>, x) => {
      if (x.group) {
        (rv[x.group] = rv[x.group] || {
          types: [],
          group: x.group,
        }).types.push(x);
      }
      return rv;
    }, {});
  }

  function groupAvailable(group: Group) {
    let res = true;
    group.types.forEach((t) => {
      if (
        attachees.find((a) => {
          return a.what == t.id;
        })
      ) {
        res = false;
      }
    });
    return res;
  }

  let allgroups = Object.values(
    groupBy(
      filterValues(types, (t) => t.group != undefined && t.main == gear.what),
    ),
  );

  let groups = allgroups.filter(groupAvailable);

  // Vendor needs to be set for any enabled group
  $: disabled = !groups.reduce((r: boolean, v: Group) => {
    return r && (!v.enabled || (v.enabled && v.vendor != ""));
  }, true);

  async function attachPart(part: Part | void, hook: number) {
    if (!part) throw "Wizard: part create failed";
    await part.attach(part.purchase, true, gear.id!, hook);
  }

  async function installPart(newpart: Part, hook: number) {
    disabled = true;
    await newpart.create().then((p) => attachPart(p, hook));
  }

  function setGroup(g: Group) {
    if (!g.enabled) return;

    let p: Part = new Part(gear);

    p.id = undefined;
    p.name = g.vendor + " " + g.model;
    p.vendor = g.vendor;
    p.model = g.model;
    g.types.forEach((t) => {
      p.what = t.id;
      t.hooks.forEach((h) => {
        installPart(p, h);
      });
    });
  }

  function save() {
    groups.forEach((g) => {
      if (g.enabled) setGroup(g);
    });
    groups = groups.filter((g) => !g.enabled);
    show_button = true;
  }
  let show_button = groups.length != allgroups.length;
</script>

{#if !gear.disposed_at && groups.length > 0}
  <div>
    {#if show_button}
      <Button color="primary" onclick={() => (show_button = false)}>
        Add more component groups
      </Button>
    {:else}
      <Table>
        <tbody>
          <tr>
            <th colspan="80"> Add components groups: </th>
          </tr>
          {#each groups as g, i}
            <tr>
              <th>
                <Switch bind:checked={g.enabled}>
                  {g.group}:
                </Switch>
              </th>
              <td>
                <ButtonGroup>
                  <Input
                    type="text"
                    id="inputBrand"
                    bind:value={g.vendor}
                    placeholder="Brand"
                    disabled={!g.enabled}
                  />
                  <Input
                    type="text"
                    id="inputModel"
                    bind:value={g.model}
                    placeholder="Model"
                    disabled={!g.enabled}
                  />
                </ButtonGroup>
              </td>
            </tr>
          {/each}
        </tbody>
      </Table>
      <Button {disabled} onclick={save}>Set</Button>
    {/if}
  </div>
{/if}
