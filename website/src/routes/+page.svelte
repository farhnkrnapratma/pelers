<script lang="ts">
    import { renderMarkdown, stripFrontmatter } from '$lib/markdown';
    import { sources } from '$lib/content';
    import examples from '$lib/generated/examples.json';

    const doc = stripFrontmatter(sources.home);
    const html = renderMarkdown(doc.body);
</script>

<svelte:head>
    <title>{doc.title}</title>
    <meta name="description" content={doc.description} />
</svelte:head>

<section class="doc-prose">
    <div class="mb-8 border border-line bg-surface-subtle p-4">
        {@html examples.homeExample}
    </div>
    {@html html}
</section>
