<script lang="ts">
  import type { HTMLSelectAttributes } from "svelte/elements";

  export type SelectOption = {
    value: string;
    label: string;
    disabled?: boolean;
  };

  type Props = Omit<HTMLSelectAttributes, "value"> & {
    label?: string;
    value?: string;
    options: SelectOption[];
    error?: string | null;
    density?: "default" | "compact";
  };

  let {
    label,
    value = $bindable(""),
    options,
    error = null,
    density = "default",
    class: className = "",
    ...rest
  }: Props = $props();
</script>

<label class="grid gap-1.5 text-[12px]">
  {#if label}
    <span class="text-[12px] text-text-subtle">{label}</span>
  {/if}
  <select
    {...rest}
    class={`w-full rounded-md border bg-surface-raised text-[13px] text-text outline-none transition-colors duration-75 ${
      error ? "border-danger focus:border-danger" : "border-border focus:border-accent"
    } ${density === "compact" ? "h-7 px-2" : "h-8 px-2.5"} ${className}`}
    bind:value
  >
    {#each options as option}
      <option value={option.value} disabled={option.disabled}>{option.label}</option>
    {/each}
  </select>
  {#if error}
    <span class="text-xs text-danger">{error}</span>
  {/if}
</label>
