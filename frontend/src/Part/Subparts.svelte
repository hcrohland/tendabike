<script lang="ts">
  import { types, Type } from "../lib/types";
  import { filterValues, by } from "../lib/mapable";
  import PartCard from "./PartCard.svelte";
  import { Part } from "../lib/part";
  import { Attachment } from "../lib/attachment";
  import Wizard from "./Wizard.svelte";

  interface Props {
    part: Part;
    attachees: Attachment[];
  }

  let { part, attachees }: Props = $props();

  type TreeNode = {
    attachments: Attachment[];
    prefix: string;
    type: Type;
    children: TreeNode[];
  };

  function buildTree(
    hook: Type,
    attachees: Attachment[],
    prefix: string,
  ): TreeNode[] {
    const typeList = filterValues(types, (a: Type) =>
      a.hooks.includes(hook.id),
    ).sort((a: Type, b: Type) => a.order - b.order);

    return typeList.map((type) => {
      let attachments = attachees.filter((a: Attachment) => {
        return a.hook == hook.id && a.what == type.id;
      });
      attachments.sort(by("attached"));

      const children =
        attachments.length > 0
          ? buildTree(type, attachees, "")
          : buildTree(type, attachees, type.prefix);

      return { attachments, prefix, type, children };
    });
  }
</script>

{#if attachees.length > 0}
  <div class="flex flex-col gap-3">
    {#each buildTree(part.type(), attachees, "") as node (node.type.id)}
      <PartCard {...node} />
    {/each}
  </div>
{/if}

{#if part.isGear()}
  <Wizard gear={part} {attachees} />
{/if}
