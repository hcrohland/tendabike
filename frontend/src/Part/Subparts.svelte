<script lang="ts">
  import { Type } from "../lib/types";

  import { Table } from "@sveltestrap/sveltestrap";
  import { filterValues, by } from "../lib/mapable";
  import { types } from "../lib/store";
  import SubType from "./SubType.svelte";
  import Wizard from "./Wizard.svelte";
  import { Part } from "./part";
  import { attachments, Attachment } from "../Attachment/attachment";

  export let gear: Part;
  export let hook: Type;

  $: attachees = filterValues($attachments, (a) => a.gear == gear.id);

  type MyList = {
    attachments: Attachment[];
    prefix: string;
    level: number;
    type: Type;
    hook: Type;
  };

  function buildList(
    list: MyList[],
    hook: Type,
    attachees: Attachment[],
    level: number,
    prefix: string,
  ) {
    const typeList = filterValues(types, (a: Type) =>
      a.hooks.includes(hook.id),
    ).sort((a: Type, b: Type) => a.order - b.order);
    typeList.forEach((type) => {
      let attachments = attachees.filter((a: Attachment) => {
        return a.hook == hook.id && a.what == type.id;
      });
      attachments.sort(by("attached"));
      list.push({ attachments, prefix, level, type, hook });
      if (attachments.length > 0) {
        buildList(list, type, attachees, level + 1, "");
      } else {
        buildList(list, type, attachees, level, type.prefix);
      }
    });
    return list;
  }
</script>

{#if attachees.length > 0}
  <Table responsive hover>
    <thead>
      <SubType />
    </thead>
    <tbody>
      {#each buildList([], hook, attachees, 0, "") as item (item.hook.id + "." + item.type.id)}
        <SubType {...item} />
      {/each}
    </tbody>
  </Table>
{/if}
{#if gear.what == types[gear.what].main}
  <Wizard {gear} {attachees} />
{/if}
