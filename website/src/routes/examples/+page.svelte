<script lang="ts">
    import examples from '$lib/generated/examples.json';
    import sources from '$lib/content/examples.json';

    const items = [
        { id: 'homeExample', title: 'Flowchart' },
        { id: 'themeExample', title: 'Themed flowchart' },
    ];
</script>

<svelte:head>
    <title>Examples · Pelers</title>
</svelte:head>

<div class="doc-prose">
    <h1>Examples</h1>
    <p class="text-fg-muted">Rendered at build time with the Pelers binding.</p>
    {#each items as item (item.id)}
        <h2>{item.title}</h2>
        <div class="border border-line bg-surface-subtle p-4">
            {@html examples[item.id as keyof typeof examples]}
        </div>
        <figure class="doc-code">
            <figcaption><span>mermaid</span></figcaption>
            <pre><code>{(sources as Record<string, string>)[item.id]}</code></pre>
        </figure>
    {/each}
</div>
