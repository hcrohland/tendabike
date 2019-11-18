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

    .param {
        font-weight: bold;
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
     is a <span class="param">{part.vendor} {part.model}</span> 
     which you used <span class=param>{part.count}</span> times 
     for <span class="param">{Math.floor(part.time /3600)}:{Math.floor(part.time/60)%60}</span> hours
     <p> You covered <span class="param">{(part.distance / 1000).toFixed(1)}</span> km climbing <span class="param">{part.climb}</span> and descending <span class="param">{part.descend}</span> meters </p>
    </CardBody>
    </div>
    <!-- </Collapse> -->
    {/if}
</Card>
