<script>
    import {getContext} from 'svelte';
    import Gear from './Gear.svelte';
    import {Card, CardBody, CardFooter, CardHeader,
    			CardImg, CardSubtitle, CardText, CardTitle, Badge} from 'sveltestrap';
    export let group;
    export let parts;

    const { getTypes } = getContext("types");
    const types = getTypes();
</script>



<Card>
    <CardHeader>
        <h3> {types[group].name}s </h3>
    </CardHeader>
    <CardBody>
    {#each parts as part}
         <Gear {part} />
    {/each}
    </CardBody>
</Card>
