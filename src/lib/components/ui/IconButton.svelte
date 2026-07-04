<script lang="ts">
  import type { HTMLButtonAttributes } from "svelte/elements";
  import type { Snippet } from "svelte";

  type Variant = "primary" | "secondary" | "danger" | "ghost";
  type Size = "sm" | "md";

  type Props = HTMLButtonAttributes & {
    label: string;
    variant?: Variant;
    size?: Size;
    children?: Snippet;
  };

  let {
    label,
    variant = "secondary",
    size = "md",
    disabled = false,
    type = "button",
    title,
    class: className = "",
    children,
    ...rest
  }: Props = $props();

  const variantClass: Record<Variant, string> = {
    primary: "bg-accent text-canvas hover:bg-accent-hover disabled:bg-surface-hover",
    secondary:
      "border border-border text-text-muted hover:bg-surface-hover hover:text-text disabled:text-text-subtle",
    danger: "border border-danger/30 text-danger hover:bg-danger/10 disabled:text-text-subtle",
    ghost: "text-text-subtle hover:bg-surface-hover hover:text-text disabled:text-text-subtle",
  };

  const sizeClass: Record<Size, string> = {
    sm: "h-7 w-7 rounded-[8px]",
    md: "h-8 w-8 rounded-md",
  };
</script>

<button
  {...rest}
  {type}
  {disabled}
  aria-label={label}
  title={title ?? label}
  class={`grid place-items-center transition-colors duration-75 disabled:cursor-not-allowed ${variantClass[variant]} ${sizeClass[size]} ${className}`}
>
  {@render children?.()}
</button>
