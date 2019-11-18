<script>
    import Gear from './Gear.svelte';
    export let parts;
</script>

<div class="card">
    <div class="cardbody">
        {#each parts as part (part.id)}
            <Gear {part} />
        {/each}
    </div>
</div>
