<script lang="ts">
  import { Input, ButtonGroup, Button } from "flowbite-svelte";
  import { types, Type, localizeGroupName } from "../lib/types.svelte";
  import { Attachment } from "../lib/attachment";
  import { type Map, filterValues } from "../lib/mapable.svelte";
  import Card from "../Widgets/Card.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import { Part } from "../lib/part";
  import { m } from "../../paraglide/messages";

  let { gear, attachees }: { gear: Part; attachees: Attachment[] } = $props();

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

  let groups = $state(allgroups.filter(groupAvailable));

  let disabled = $state(true);

  $effect(() => {
    disabled = !groups.reduce((r: boolean, v: Group) => {
      return r && (!v.enabled || (v.enabled && v.vendor != ""));
    }, true);
  });

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

  let show_button = $state(allgroups.length !== groups.length);

  $effect(() => {
    show_button = allgroups.length !== groups.length;
  });

  function save() {
    groups.forEach((g) => {
      if (g.enabled) setGroup(g);
    });
  }
</script>

{#if !gear.disposed_at && groups.length > 0}
  <div>
    {#if show_button}
      <Button
        color="primary"
        class="mt-8"
        onclick={() => (show_button = false)}
      >
        {m.wizard_add_more_groups()}
      </Button>
    {:else}
      <p
        class="mb-2 text-sm font-semibold text-secondary-900 dark:text-secondary-100"
      >
        {m.wizard_add_groups()}
      </p>
      <div class="flex flex-col gap-1 sm:gap-2">
        {#each groups as g}
          <Card>
            <div class="flex items-center gap-2 p-1 sm:p-2">
              <Switch bind:checked={g.enabled}>
                {localizeGroupName(g.group)}:
              </Switch>
              <div class="flex-1">
                <ButtonGroup>
                  <Input
                    type="text"
                    bind:value={g.vendor}
                    placeholder={m.partform_brand()}
                    disabled={!g.enabled}
                  />
                  <Input
                    type="text"
                    bind:value={g.model}
                    placeholder={m.partform_model()}
                    disabled={!g.enabled}
                  />
                </ButtonGroup>
              </div>
            </div>
          </Card>
        {/each}
      </div>
      <Button {disabled} class="mt-2" onclick={save}>{m.action_set()}</Button>
    {/if}
  </div>
{/if}
