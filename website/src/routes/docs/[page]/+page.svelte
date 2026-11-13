<script lang="ts">
    import { renderMarkdown, stripFrontmatter } from '$lib/markdown';
    let { data } = $props();
    const doc = $derived(stripFrontmatter(data.source));
    const html = $derived(renderMarkdown(doc.body));
</script>

<svelte:head>
    <title>{doc.title} · Pelers</title>
    <meta name="description" content={doc.description} />
</svelte:head>

<article class="doc-prose">
    <h1>{doc.title}</h1>
    <p class="text-fg-muted">{doc.description}</p>
    {@html html}
</article>
