<script lang="ts">
  import Icon from "$lib/components/ui/Icon.svelte";

  type SectionId = string;

  const sectionNavItems = [
    { id: "settings-general", label: "General" },
    { id: "settings-environment", label: "Environment" },
    { id: "settings-processes", label: "Processes" },
  ] as const;

  type Props = {
    activeSection: string;
    onNavigate: (sectionId: string) => void;
    onClose: () => void;
  };

  let { activeSection, onNavigate, onClose }: Props = $props();

  function navClass(sectionId: string) {
    return activeSection === sectionId
      ? "bg-surface-raised/70 text-text"
      : "text-text-subtle hover:bg-surface-hover/70 hover:text-text";
  }

  function handleNavigate(event: MouseEvent, sectionId: SectionId) {
    event.preventDefault();
    onNavigate(sectionId);
  }
</script>

<aside class="min-h-0 border-r border-border bg-surface">
  <div class="flex h-full min-h-0 flex-col px-3 py-3">
    <nav aria-label="Menu sections">
      <div class="grid gap-1">
        <a
          class="settings-nav-item inline-flex items-center gap-2 rounded-md px-3 py-1.5 text-[13px] text-text-subtle transition-colors duration-75 hover:bg-surface-hover/70 hover:text-text"
          href="#close"
          onclick={(event) => {
            event.preventDefault();
            onClose();
          }}
        >
          <Icon name="back" size="sm" />
          Go back</a
        >

        {#each sectionNavItems as section (section.id)}
          <a
            class="settings-nav-item rounded-md px-3 py-1.5 text-[13px] transition-colors duration-75 {navClass(section.id)}"
            href={`#${section.id}`}
            aria-current={activeSection === section.id ? "page" : undefined}
            onclick={(event) => handleNavigate(event, section.id)}
            >{section.label}</a
          >
        {/each}
      </div>
    </nav>
  </div>
</aside>
