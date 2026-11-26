<script lang="ts">
  import { onMount } from "svelte";
  import samples from "#lib/generated/samples.json";
  import content from "#lib/content/examples.json";

  type Lazy = typeof import("@farhnkrnapratma/pelers/lazy");
  type Failure = {
    message: string;
    line?: number;
    column?: number;
    snippet?: string;
  };

  const sampleTypes = Object.keys(samples) as (keyof typeof samples)[];

  let engine = $state<Lazy>();
  let loadError = $state("");
  let source = $state(content.playgroundExample);
  let sample = $state("");
  let fontSize = $state(16);
  let maxWidth = $state(0);
  let padding = $state(8);
  let autoDirection = $state(false);
  let dark = $state(false);
  let svg = $state("");
  let summary = $state("");
  let failure = $state<Failure | null>(null);
  let notice = $state("");

  const encode = (text: string): string => {
    let binary = "";
    for (const byte of new TextEncoder().encode(text)) {
      binary += String.fromCharCode(byte);
    }
    return btoa(binary)
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=+$/, "");
  };

  const decode = (value: string): string =>
    new TextDecoder().decode(
      Uint8Array.from(atob(value.replace(/-/g, "+").replace(/_/g, "/")), (c) =>
        c.charCodeAt(0),
      ),
    );

  onMount(() => {
    const shared = new URLSearchParams(location.hash.slice(1)).get("code");
    if (shared) {
      try {
        source = decode(shared);
      } catch {
        notice = "The shared link could not be read.";
      }
    }
    import("@farhnkrnapratma/pelers/lazy")
      .then(async (lib) => {
        await lib.load();
        engine = lib;
      })
      .catch((error: unknown) => {
        loadError = error instanceof Error ? error.message : String(error);
      });
  });

  $effect(() => {
    const lib = engine;
    if (!lib) return;
    const text = source;
    const options = {
      fontSize,
      padding,
      autoDirection,
      ...(maxWidth > 0 ? { maxWidth } : {}),
    };
    const timer = setTimeout(() => {
      try {
        const started = performance.now();
        const result = lib.render(text, options);
        const elapsed = performance.now() - started;
        svg = result.svg;
        summary = `${result.type} · ${Math.round(result.width)} × ${Math.round(result.height)} · ${elapsed.toFixed(1)} ms`;
        failure = null;
      } catch (error) {
        const { message, line, column, snippet } = error as Failure;
        failure = { message, line, column, snippet };
      }
      history.replaceState(null, "", `#code=${encode(text)}`);
    }, 120);
    return () => clearTimeout(timer);
  });

  const pickSample = () => {
    if (sample) source = samples[sample as keyof typeof samples];
  };

  const flash = (text: string) => {
    notice = text;
    setTimeout(() => (notice = ""), 1500);
  };

  const copySvg = async () => {
    await navigator.clipboard?.writeText(svg);
    flash("SVG copied");
  };

  const copyLink = async () => {
    await navigator.clipboard?.writeText(location.href);
    flash("Link copied");
  };

  const download = () => {
    const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = "diagram.svg";
    link.click();
    URL.revokeObjectURL(url);
  };
</script>

<svelte:head>
  <title>Playground · Pelers</title>
  <meta
    name="description"
    content="Render Mermaid diagrams to SVG in the browser with Pelers."
  />
</svelte:head>

<div class="doc-prose">
  <h1>Pelers Playground</h1>
  <p class="text-fg-muted">
    Edit Mermaid text and see the SVG as you type. Everything runs in your
    browser with the WebAssembly build of Pelers.
  </p>
</div>

<div class="mt-6 flex flex-wrap items-end gap-4 text-sm">
  <label class="flex flex-col gap-1">
    <span class="text-fg-muted">Example</span>
    <select
      bind:value={sample}
      onchange={pickSample}
      class="border border-line bg-surface px-2 py-1"
    >
      <option value="">Choose a diagram type</option>
      {#each sampleTypes as type (type)}
        <option value={type}>{type}</option>
      {/each}
    </select>
  </label>
  <label class="flex flex-col gap-1">
    <span class="text-fg-muted">Font size</span>
    <input
      type="number"
      min="8"
      max="40"
      bind:value={fontSize}
      class="w-20 border border-line bg-surface px-2 py-1"
    />
  </label>
  <label class="flex flex-col gap-1">
    <span class="text-fg-muted">Max width (0 = none)</span>
    <input
      type="number"
      min="0"
      step="40"
      bind:value={maxWidth}
      class="w-28 border border-line bg-surface px-2 py-1"
    />
  </label>
  <label class="flex flex-col gap-1">
    <span class="text-fg-muted">Padding</span>
    <input
      type="number"
      min="0"
      max="64"
      bind:value={padding}
      class="w-20 border border-line bg-surface px-2 py-1"
    />
  </label>
  <label class="flex items-center gap-2 py-1">
    <input type="checkbox" bind:checked={autoDirection} />
    <span>Auto direction</span>
  </label>
  <label class="flex items-center gap-2 py-1">
    <input type="checkbox" bind:checked={dark} />
    <span>Dark theme</span>
  </label>
</div>

<div class="mt-4 grid gap-4 lg:grid-cols-2">
  <label class="flex flex-col gap-1">
    <span class="text-sm text-fg-muted">Mermaid</span>
    <textarea
      bind:value={source}
      spellcheck="false"
      autocapitalize="off"
      autocomplete="off"
      rows="22"
      class="w-full resize-y border border-line bg-surface p-3 font-mono text-code leading-relaxed"
    ></textarea>
  </label>

  <section class="flex min-w-0 flex-col gap-2" aria-label="Preview">
    <div class="flex flex-wrap items-center gap-2 text-sm">
      <span class="text-fg-muted" aria-live="polite"
        >{engine ? summary : loadError || "Loading the renderer…"}</span
      >
      <span class="ml-auto flex gap-2">
        <button
          type="button"
          onclick={copySvg}
          disabled={!svg}
          class="border border-line px-2 py-1 hover:bg-surface-subtle"
          >Copy SVG</button
        >
        <button
          type="button"
          onclick={download}
          disabled={!svg}
          class="border border-line px-2 py-1 hover:bg-surface-subtle"
          >Download</button
        >
        <button
          type="button"
          onclick={copyLink}
          class="border border-line px-2 py-1 hover:bg-surface-subtle"
          >Copy link</button
        >
      </span>
    </div>
    {#if notice}
      <p class="text-sm text-fg-accent" role="status">{notice}</p>
    {/if}
    {#if failure}
      <div
        class="border border-line bg-surface-subtle p-3 text-sm"
        role="alert"
      >
        <p class="font-semibold">
          {failure.message}{#if failure.line}
            <span class="font-normal text-fg-muted">
              (line {failure.line}, column {failure.column})</span
            >{/if}
        </p>
        {#if failure.snippet}
          <pre
            class="mt-2 overflow-x-auto font-mono text-code">{failure.snippet}</pre>
        {/if}
      </div>
    {/if}
    <div
      class="preview flex min-h-80 items-start justify-center overflow-auto border border-line p-4"
      class:preview-dark={dark}
    >
      {@html svg}
    </div>
  </section>
</div>

<style>
  .preview {
    background: #fff;
  }

  .preview-dark {
    background: #1c1b1a;
    --pelers-bg: #1c1b1a;
    --pelers-fg: #f2f0e5;
    --pelers-muted: #b7b5ac;
    --pelers-line: #f2f0e5;
    --pelers-surface: #2a2926;
    --pelers-surface-alt: #343331;
    --pelers-border: #575653;
  }
</style>
