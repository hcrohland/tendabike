<script>
    import {getContext} from 'svelte';
    import Gear from './Gear.svelte';
    import {types} from './store.js';
    import {Card, CardBody, CardFooter, CardHeader,
    			CardImg, CardSubtitle, CardText, CardTitle, Badge} from 'sveltestrap';
    export let parts;

</script>


<Card>
    <!-- <CardHeader>
        <h3> {$types[group].name}s </h3>
    </CardHeader> -->
    <CardBody>
        {#each parts as part (part.id)}
            <Gear {part} />
        {/each}
    </CardBody>
</Card>
