<script>
    import {Button, Card, CardBody, CardFooter, CardHeader,
                CardImg, CardSubtitle, CardText, CardTitle, Badge, Collapse} from 'sveltestrap';
    import { slide } from 'svelte/transition';

    export let part;

    let isOpen = false;
</script>

<style>
    .header:hover {
        background-color: lightgray;
        /* margin-bottom: 0.3rem; */
    }
</style>

<Card style="margin: 1rem">
<div class="header">
        <!-- <Button block> -->
    <CardHeader on:click={() => (isOpen = !isOpen)}>
         <h4> {part.name} </h4>
    </CardHeader>
         <!-- </Button> -->
</div>
    {#if isOpen}
    <!-- <Collapse {isOpen}> -->
    <div transition:slide>
    <CardBody>
     is a {part.vendor} {part.model}
    </CardBody>
    </div>
    <!-- </Collapse> -->
    {/if}
</Card>
