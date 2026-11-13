<script lang="ts">
    import { page } from '$app/state';
</script>

<svelte:head>
    <title>{page.status} · Pelers</title>
</svelte:head>

<div class="doc-prose">
    <h1>{page.status}</h1>
    <p>{page.error?.message ?? 'Something went wrong.'}</p>
    <p><a href="/">Back to home</a></p>
</div>
