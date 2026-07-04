<script lang="ts">
  import type { Snippet } from "svelte";
  import AppWindow from "$lib/components/ui/layout/AppWindow.svelte";
  import MainPanel from "$lib/components/ui/layout/MainPanel.svelte";
  import Sidebar from "$lib/components/ui/layout/Sidebar.svelte";
  import Toast from "$lib/components/ui/Toast.svelte";

  type Props = {
    titleBar: Snippet;
    processList: Snippet;
    children: Snippet;
  };

  let { titleBar, processList, children }: Props = $props();
</script>

{#snippet topFrame()}
  {@render titleBar()}
{/snippet}

{#snippet sidebar()}
  <Sidebar>
    {@render processList()}
  </Sidebar>
{/snippet}

{#snippet main()}
  <MainPanel>
    {@render children()}
  </MainPanel>
{/snippet}

<main class="h-full min-h-0 overflow-hidden bg-canvas text-text">
  <AppWindow {topFrame} {sidebar} {main} />
  <Toast />
</main>
