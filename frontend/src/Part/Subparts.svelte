<script lang="ts">
  import { types, Type } from "../lib/types";
  import { Table } from "@sveltestrap/sveltestrap";
  import { filterValues, by } from "../lib/mapable";
  import SubType from "./SubType.svelte";
  import { Part } from "../lib/part";
  import { Attachment } from "../lib/attachment";
  import Wizard from "./Wizard.svelte";

  export let part: Part;
  export let attachees: Attachment[];

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
    // the list of types that can be attached to the hook
    const typeList = filterValues(types, (a: Type) =>
      a.hooks.includes(hook.id),
    ).sort((a: Type, b: Type) => a.order - b.order);
    typeList.forEach((type) => {
      // the list of attachments at the hook
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
      {#each buildList([], part.type(), attachees, 0, "") as item (item.hook.id + "." + item.type.id)}
        <SubType {...item} />
      {/each}
    </tbody>
  </Table>
{/if}
{#if part.isGear()}
  <Wizard gear={part} {attachees} />
{/if}
