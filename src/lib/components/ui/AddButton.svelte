<script lang="ts">
  import Button from "./Button.svelte";
  import Icon from "./Icon.svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  type Props = {
    label?: string;
    onclick: () => void;
    disabled?: boolean;
    class?: string;
  } & Omit<HTMLButtonAttributes, "class" | "children">;

  let { label = "New", onclick, disabled = false, class: cls = "", ...rest }: Props = $props();
</script>

<Button size="xs" variant="ghost" {onclick} {disabled} class={cls} {...rest}>
  <Icon name="plus" size="sm" />
  <span>{label}</span>
</Button>
