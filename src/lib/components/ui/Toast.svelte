<script lang="ts">
  import { toastStore, type ToastType } from "$lib/stores/toast.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";

  const toasts = $derived(toastStore.toasts);

  const iconByType: Record<ToastType, string> = {
    success: "\u2713",
    info: "\u24D8",
    error: "\u2715",
  };

  const borderByType: Record<ToastType, string> = {
    success: "border-success/30",
    info: "border-blue-400/30",
    error: "border-danger/30",
  };

  const textByType: Record<ToastType, string> = {
    success: "text-success",
    info: "text-blue-400",
    error: "text-danger",
  };
</script>

{#if toasts.length > 0}
  <div
    class="pointer-events-none fixed bottom-4 right-4 z-[60] flex flex-col-reverse gap-2"
  >
    {#each toasts as toast (toast.id)}
      <div
        class="pointer-events-auto flex items-start gap-2 rounded-lg border bg-surface-raised px-3 py-2 text-[13px] shadow-lg select-none animate-toast-in {borderByType[
          toast.type
        ]}"
        style="max-width: 320px"
      >
        <span class="{textByType[toast.type]} mt-0.5 text-sm"
          >{iconByType[toast.type]}</span
        >
        <div class="min-w-0 flex-1">
          <div class="text-text">{toast.message}</div>
          {#if toast.detail}
            <div class="mt-0.5 text-[11px] text-text-muted">{toast.detail}</div>
          {/if}
        </div>
        <button
          type="button"
          class="ml-1 shrink-0 rounded-sm text-text-subtle transition-colors duration-75 hover:text-text"
          aria-label="Dismiss"
          onclick={() => toastStore.dismiss(toast.id)}
        >
          <Icon name="close" size="xs" />
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  @keyframes toast-in {
    from {
      transform: translateY(6px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .animate-toast-in {
    animation: toast-in 90ms ease-out;
  }

  @media (prefers-reduced-motion: reduce) {
    .animate-toast-in {
      animation: none;
    }
  }
</style>
