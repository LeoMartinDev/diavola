<script lang="ts">
  import Icon from "./Icon.svelte";
  import IconButton from "./IconButton.svelte";
  import { tick } from "svelte";
  import { fly, scale } from "svelte/transition";
  import type { Snippet } from "svelte";

  type Size = "sm" | "md" | "lg" | "xl";
  type Variant = "modal" | "panel";

  type Props = {
    open: boolean;
    title: string;
    description?: string;
    size?: Size;
    variant?: Variant;
    onClose: () => void;
    closeOnOverlay?: boolean;
    children?: Snippet;
    footer?: Snippet;
  };

  let {
    open,
    title,
    description,
    size = "md",
    variant = "modal",
    onClose,
    closeOnOverlay = true,
    children,
    footer,
  }: Props = $props();

  let panel = $state<HTMLElement | null>(null);
  let titleId = $state(`dialog-title-${crypto.randomUUID()}`);
  let descriptionId = $state(`dialog-description-${crypto.randomUUID()}`);
  let previouslyFocused = $state<HTMLElement | null>(null);
  let wasOpen = $state(false);

  const prefersReducedMotion =
    typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia("(prefers-reduced-motion: reduce)").matches
      : false;

  const sizeClass: Record<Size, string> = {
    sm: "w-[min(440px,calc(100vw-32px))]",
    md: "w-[min(560px,calc(100vw-32px))]",
    lg: "w-[min(840px,calc(100vw-32px))]",
    xl: "w-[min(1040px,calc(100vw-32px))]",
  };

  const layoutClass = $derived(
    variant === "panel"
      ? "pointer-events-none items-center justify-center p-4 sm:items-start sm:justify-end sm:pt-[46px]"
      : "items-center justify-center p-4",
  );

  const overlayClass = $derived(
    variant === "panel" ? "pointer-events-none bg-transparent" : "bg-black/40",
  );

  const panelClass = $derived(
    variant === "panel"
      ? `pointer-events-auto sm:h-[calc(100vh-56px)] sm:max-h-[calc(100vh-56px)] ${sizeClass[size]}`
      : sizeClass[size],
  );

  const focusableSelector = [
    "a[href]",
    "button:not([disabled])",
    "textarea:not([disabled])",
    "input:not([disabled])",
    "select:not([disabled])",
    "[tabindex]:not([tabindex='-1'])",
  ].join(",");

  function focusableElements() {
    if (!panel) {
      return [];
    }
    return Array.from(
      panel.querySelectorAll<HTMLElement>(focusableSelector),
    ).filter(
      (element) => !element.hasAttribute("disabled") && element.tabIndex !== -1,
    );
  }

  async function focusInitial() {
    await tick();
    const elements = focusableElements();
    const first =
      variant === "panel"
        ? (elements.find(
            (element) => !element.hasAttribute("data-dialog-dismiss"),
          ) ?? panel)
        : (elements[0] ?? panel);
    first?.focus();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) {
      return;
    }

    const target = event.target as Node | null;
    const focusInsidePanel = !!panel && !!target && panel.contains(target);

    if (event.key === "Escape") {
      if (variant === "modal" || focusInsidePanel) {
        event.preventDefault();
        onClose();
      }
      return;
    }

    if (variant === "panel") {
      return;
    }

    if (event.key !== "Tab") {
      return;
    }

    const elements = focusableElements();
    if (elements.length === 0) {
      event.preventDefault();
      panel?.focus();
      return;
    }

    const first = elements[0];
    const last = elements[elements.length - 1];
    const active = document.activeElement;
    if (event.shiftKey && active === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  $effect(() => {
    if (open && !wasOpen) {
      previouslyFocused =
        document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null;
      wasOpen = true;
      void focusInitial();
      return;
    }
    if (!open && wasOpen) {
      wasOpen = false;
      previouslyFocused?.focus();
      previouslyFocused = null;
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <div class={`fixed inset-0 z-50 flex ${layoutClass}`}>
    {#if variant === "modal"}
      <button
        type="button"
        class={`absolute inset-0 ${overlayClass}`}
        transition:fly={{ duration: prefersReducedMotion ? 0 : 75 }}
        aria-label="Close dialog"
        tabindex="-1"
        onclick={() => closeOnOverlay && onClose()}
      ></button>
    {:else}
      <div
        class={`absolute inset-0 ${overlayClass}`}
        transition:fly={{ duration: prefersReducedMotion ? 0 : 75 }}
        aria-hidden="true"
      ></div>
    {/if}
    <div
      bind:this={panel}
      class={`relative z-10 flex max-h-[calc(100vh-44px)] min-h-0 flex-col overflow-hidden rounded-xl border border-border bg-surface text-text shadow-2xl outline-none ${panelClass}`}
      transition:scale={{
        duration: prefersReducedMotion ? 0 : 75,
        start: 0.97,
      }}
      role="dialog"
      data-dialog-variant={variant}
      aria-modal={variant === "modal" ? "true" : undefined}
      aria-labelledby={titleId}
      aria-describedby={description ? descriptionId : undefined}
      tabindex="-1"
    >
      <header
        class={`border-b border-border px-5 ${variant === "panel" ? "py-3" : "py-3.5"}`}
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0 flex-1">
            <h2 id={titleId} class="text-sm font-semibold">{title}</h2>
            {#if description}
              <p
                id={descriptionId}
                class="mt-1 text-xs leading-5 text-text-subtle"
              >
                {description}
              </p>
            {/if}
          </div>

          {#if variant === "panel"}
            <IconButton
              label="Close panel"
              variant="ghost"
              class="h-7 w-7 shrink-0 rounded-sm border border-transparent"
              data-dialog-dismiss
              onclick={() => onClose()}
            >
              <Icon name="close" size="sm" />
            </IconButton>
          {/if}
        </div>
      </header>

      <div class="min-h-0 flex-1 overflow-hidden">
        {@render children?.()}
      </div>

      {#if footer}
        <footer class="border-t border-border px-5 py-3.5">
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}
