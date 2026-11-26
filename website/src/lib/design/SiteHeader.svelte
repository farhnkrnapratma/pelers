<script lang="ts">
  import { page } from "$app/state";
  import { docsPages, repositoryUrl } from "#lib/site";
  import Brand from "./Brand.svelte";

  const links = [
    { href: "/", label: "Home" },
    { href: "/examples", label: "Examples" },
    ...docsPages.map((doc) => ({ href: `/docs/${doc.id}`, label: doc.title })),
    { href: "/playground", label: "Playground" },
  ];
</script>

<header class="border-b border-line-subtle bg-surface">
  <nav
    class="mx-auto flex max-w-5xl flex-wrap items-center gap-x-6 gap-y-2 px-4 py-3"
    aria-label="Primary"
  >
    <a
      href="/"
      class="inline-flex h-8 items-center font-title text-lg font-bold leading-none no-underline text-fg"
      aria-label="Pelers home"><Brand size="1.6em" /></a
    >
    <ul class="flex h-8 flex-wrap items-center gap-x-4 gap-y-1 text-sm">
      {#each links as link (link.href)}
        <li class="flex h-8 items-center">
          <a
            href={link.href}
            class="inline-flex h-8 items-center no-underline leading-none {page.url.pathname === link.href
              ? 'text-fg-accent font-semibold'
              : 'text-fg-secondary hover:text-fg'}"
            aria-current={page.url.pathname === link.href ? "page" : undefined}
            >{link.label}</a
          >
        </li>
      {/each}
    </ul>
    <a
      href={repositoryUrl}
      class="ml-auto text-sm text-fg-muted no-underline hover:text-fg"
      rel="noopener noreferrer">GitHub</a
    >
  </nav>
</header>
