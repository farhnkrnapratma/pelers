<script lang="ts">
    import '../app.css';
    import SiteHeader from '$lib/design/SiteHeader.svelte';
    import SiteFooter from '$lib/design/SiteFooter.svelte';
    import { onMount } from 'svelte';

    let { children } = $props();

    onMount(() => {
        const onClick = (event: MouseEvent) => {
            const button = (event.target as HTMLElement).closest<HTMLElement>('[data-copy-code]');
            if (!button) return;
            navigator.clipboard?.writeText(button.dataset.copyCode ?? '');
            button.textContent = 'Copied';
            setTimeout(() => (button.textContent = 'Copy'), 1200);
        };
        document.addEventListener('click', onClick);
        return () => document.removeEventListener('click', onClick);
    });
</script>

<div class="flex min-h-screen flex-col">
    <SiteHeader />
    <main class="mx-auto w-full max-w-5xl flex-1 px-4 py-8">
        {@render children()}
    </main>
    <SiteFooter />
</div>
