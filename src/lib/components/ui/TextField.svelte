<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";

  type Props = Omit<HTMLInputAttributes, "value"> & {
    label?: string;
    value?: string | number | null;
    error?: string | null;
    monospace?: boolean;
    density?: "default" | "compact";
  };

  let {
    label,
    value = $bindable(""),
    placeholder,
    error = null,
    type = "text",
    autocomplete,
    monospace = false,
    density = "default",
    class: className = "",
    ...rest
  }: Props = $props();

  const inputClass = $derived(
    `w-full rounded-md border bg-surface-raised text-[13px] text-text outline-none transition-colors duration-75 placeholder:text-text-subtle ${
      error ? "border-danger focus:border-danger" : "border-border focus:border-accent"
    } ${density === "compact" ? "h-7 px-2" : "h-8 px-2.5"} ${monospace ? "font-mono" : ""} ${className}`,
  );
</script>

<label class="grid gap-1.5 text-[12px]">
  {#if label}
    <span class="text-[12px] text-text-subtle">{label}</span>
  {/if}
  <input {...rest} {type} {placeholder} {autocomplete} class={inputClass} bind:value />
  {#if error}
    <span class="text-xs text-danger">{error}</span>
  {/if}
</label>
