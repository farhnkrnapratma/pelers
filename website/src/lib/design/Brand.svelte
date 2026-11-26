<script lang="ts">
  let { size = "1.15em", label = "Pelers" }: { size?: string; label?: string } =
    $props();
</script>

<span
  class="inline-flex h-full items-center gap-[0.3em] whitespace-nowrap leading-none"
>
  <img
    src="/favicon.svg"
    alt=""
    width="449"
    height="336"
    style="height: {size}; width: auto;"
  />{label}</span
>
