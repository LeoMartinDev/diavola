<script lang="ts">
  import Button from "$lib/components/ui/Button.svelte";
  import Dialog from "$lib/components/ui/Dialog.svelte";

  type Props = {
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    busy?: boolean;
    onConfirm: () => void | Promise<void>;
    onClose: () => void;
  };

  let {
    open,
    title,
    message,
    confirmLabel = "Confirm",
    cancelLabel = "Cancel",
    busy = false,
    onConfirm,
    onClose,
  }: Props = $props();
</script>

<Dialog {open} {title} description={message} size="sm" {onClose}>
  {#snippet footer()}
    <div class="flex justify-end gap-2">
      <Button onclick={onClose} disabled={busy}>{cancelLabel}</Button>
      <Button variant="danger" onclick={() => void onConfirm()} disabled={busy}>
        {confirmLabel}
      </Button>
    </div>
  {/snippet}
</Dialog>
