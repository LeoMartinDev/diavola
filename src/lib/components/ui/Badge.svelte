<script lang="ts">
  import type { Snippet } from "svelte";

  type Tone = "neutral" | "success" | "warning" | "danger" | "info";

  type Props = {
    tone?: Tone;
    class?: string;
    children?: Snippet;
  };

  let { tone = "neutral", class: className = "", children }: Props = $props();

  const toneClass: Record<Tone, string> = {
    neutral: "bg-surface-hover text-text-subtle",
    success: "bg-success/10 text-success",
    warning: "bg-warning/10 text-warning",
    danger: "bg-danger/10 text-danger",
    info: "bg-accent/10 text-accent",
  };
</script>

<span class={`inline-flex items-center rounded-full px-2 py-0.5 text-[11px] ${toneClass[tone]} ${className}`}>
  {@render children?.()}
</span>
