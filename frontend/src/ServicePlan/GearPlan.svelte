<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    Form,
  } from "@sveltestrap/sveltestrap";
  import ModalFooter from "../Widgets/ModalFooter.svelte";
  import { Limits, ServicePlan } from "./serviceplan";
  import { Part, parts } from "../Part/part";
  import PlanForm from "./PlanForm.svelte";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import type { Type } from "../lib/types";

  let gear: Part;
  let plan: ServicePlan | undefined;
  let isOpen = false;
  let newplan: { name?: string | undefined; limits?: Limits } = {};
  let newhook: { part: number | undefined; what: number; hook: number };

  async function createPlan() {
    let plan = new ServicePlan(
      Object.assign({ name: newplan.name }, newplan.limits, newhook),
    );
    await plan.create();
    isOpen = false;
  }

  export const newServicePlan = (part: Part) => {
    gear = part;
    isOpen = true;
  };

  const toggle = () => {
    isOpen = false;
    plan = undefined;
  };
  const sethook = (e: CustomEvent<{ type: Type; hook: number }>) => {
    newhook = { part: gear.id, what: e.detail.type.id, hook: e.detail.hook };
  };

  const setPlan = (e: CustomEvent<{ name: string; limits: Limits }>) => {
    newplan = e.detail;
  };

  $: disabled = !(
    newhook &&
    newplan &&
    newplan.name &&
    newplan.limits &&
    newplan.name.length > 0 &&
    newplan.limits.check()
  );
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    <TypeForm {gear} on:change={sethook}>New service plan for</TypeForm>
  </ModalHeader>
  <ModalBody>
    <Form>
      <PlanForm on:change={setPlan} />
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={createPlan} button={"Create"} />
</Modal>
