<script lang="ts">
    import { packageName, repositoryUrl } from '$lib/site';
</script>

<footer class="mt-16 border-t border-line-subtle bg-surface-subtle">
    <div class="mx-auto flex max-w-5xl flex-wrap items-center justify-between gap-4 px-4 py-6 text-sm text-fg-muted">
        <span>{packageName} · MIT OR Apache-2.0</span>
        <a href={repositoryUrl} rel="noopener noreferrer">Source on GitHub</a>
    </div>
</footer>
