<script lang="ts" module>
  import type { IconName } from "$lib/components/ui/Icon.svelte";

  export type MenuIcon = IconName;

  export type MenuItem = {
    label: string;
    onSelect: () => void;
    disabled?: boolean;
    danger?: boolean;
    dividerAfter?: boolean;
    icon?: MenuIcon;
  };
</script>

<script lang="ts">
  import { onMount, tick } from "svelte";
  import Icon from "$lib/components/ui/Icon.svelte";

  type Props = {
    label: string;
    items: MenuItem[];
    disabled?: boolean;
  };

  let { label, items, disabled = false }: Props = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement | null>(null);

  function enabledItems() {
    if (!root) {
      return [];
    }
    return Array.from(
      root.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'),
    ).filter((item) => !item.disabled);
  }

  async function focusFirstItem() {
    await tick();
    enabledItems()[0]?.focus();
  }

  function onPointerDown(event: MouseEvent) {
    if (!root) {
      return;
    }
    if (!root.contains(event.target as Node)) {
      open = false;
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (!open) {
      return;
    }

    if (event.key === "Escape") {
      open = false;
      return;
    }

    const items = enabledItems();
    if (items.length === 0) {
      return;
    }

    const activeIndex = items.indexOf(
      document.activeElement as HTMLButtonElement,
    );
    const currentIndex = activeIndex === -1 ? 0 : activeIndex;

    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const nextIndex =
        event.key === "ArrowDown"
          ? (currentIndex + 1) % items.length
          : (currentIndex - 1 + items.length) % items.length;
      items[nextIndex]?.focus();
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      items[0]?.focus();
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      items[items.length - 1]?.focus();
    }
  }

  onMount(() => {
    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeydown);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeydown);
    };
  });

  function choose(item: MenuItem) {
    if (item.disabled) {
      return;
    }
    open = false;
    item.onSelect();
  }

  $effect(() => {
    if (!open) {
      return;
    }

    void focusFirstItem();
  });
</script>

<div bind:this={root} class="relative">
  <button
    type="button"
    {disabled}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={label}
    title={label}
    class="grid h-7 w-7 place-items-center rounded-md text-text-muted transition-colors duration-75 hover:bg-surface-hover hover:text-text disabled:cursor-not-allowed disabled:opacity-55"
    onclick={() => (open = !open)}
  >
    <Icon name="more" size="md" />
  </button>

  {#if open}
    <div
      role="menu"
      aria-label={label}
      class="absolute right-0 top-full z-30 mt-1 min-w-[180px] overflow-hidden rounded-md border border-border bg-surface-raised py-1 shadow-2xl select-none"
    >
      {#each items as item, i (i)}
        <button
          type="button"
          role="menuitem"
          disabled={item.disabled}
          class={`flex w-full items-center px-2.5 py-1.5 text-left text-[13px] transition-colors duration-75 disabled:cursor-not-allowed disabled:opacity-40 ${
            item.disabled
              ? ""
              : item.danger
                ? "text-danger hover:bg-danger/10"
                : "text-text-muted hover:bg-surface-hover hover:text-text"
          }`}
          onclick={() => choose(item)}
        >
          <span
            class="mr-2 inline-flex h-4 w-4 shrink-0 items-center justify-center opacity-80"
            aria-hidden="true"
          >
            {#if item.icon}
              <Icon name={item.icon} size="xs" />
            {/if}
          </span>
          {item.label}
        </button>
        {#if item.dividerAfter}
          <div class="my-1 h-px bg-border"></div>
        {/if}
      {/each}
    </div>
  {/if}
</div>
