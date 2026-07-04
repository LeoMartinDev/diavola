<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";

  type Props = Omit<HTMLInputAttributes, "checked" | "type"> & {
    label: string;
    checked?: boolean;
    error?: string | null;
  };

  let {
    label,
    checked = $bindable(false),
    error = null,
    class: className = "",
    ...rest
  }: Props = $props();
</script>

<label class={`flex items-center gap-2 text-[12px] text-text-muted ${className}`}>
  <input {...rest} class="accent-accent" type="checkbox" bind:checked />
  <span>{label}</span>
</label>
{#if error}
  <span class="text-xs text-danger">{error}</span>
{/if}
