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
     is a {part.vendor} {part.model} which you used <strong>{part.count}</strong> times for <strong>{Math.floor(part.time /3600)}:{Math.floor(part.time/60)%60}</strong> hours
     <p> You climbed {part.climb} and descended {part.descend} meters </p>
    </CardBody>
    </div>
    <!-- </Collapse> -->
    {/if}
</Card>
