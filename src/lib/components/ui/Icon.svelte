<script lang="ts" module>
  export type IconName =
    | "play"
    | "stop"
    | "pause"
    | "task"
    | "service"
    | "terminal"
    | "close"
    | "restart"
    | "search"
    | "scroll-down"
    | "trash"
    | "git-branch"
    | "settings"
    | "spinner"
    | "plus"
    | "check"
    | "info"
    | "error"
    | "back"
    | "chevron-right"
    | "more"
    | "edit"
    | "copy"
    | "clear"
    | "sun"
    | "moon"
    | "monitor"
    | "asterisk"
    | "case-sensitive";
</script>

<script lang="ts">
  import {
    Play,
    Square,
    Pause,
    Zap,
    Activity,
    Terminal,
    X,
    RotateCcw,
    Search,
    ChevronDown,
    Trash2,
    GitBranch,
    Settings,
    LoaderCircle,
    Plus,
    Check,
    Info,
    CircleX,
    ChevronLeft,
    ChevronRight,
    EllipsisVertical,
    Pencil,
    Copy,
    Trash,
    Sun,
    Moon,
    Monitor,
    Asterisk,
    CaseSensitive,
  } from "@lucide/svelte";

  const iconMap: Record<IconName, typeof Play> = {
    play: Play,
    stop: Square,
    pause: Pause,
    task: Zap,
    service: Activity,
    terminal: Terminal,
    close: X,
    restart: RotateCcw,
    search: Search,
    "scroll-down": ChevronDown,
    trash: Trash2,
    "git-branch": GitBranch,
    settings: Settings,
    spinner: LoaderCircle,
    plus: Plus,
    check: Check,
    info: Info,
    error: CircleX,
    back: ChevronLeft,
    "chevron-right": ChevronRight,
    more: EllipsisVertical,
    edit: Pencil,
    copy: Copy,
    clear: Trash,
    sun: Sun,
    moon: Moon,
    monitor: Monitor,
    asterisk: Asterisk,
    "case-sensitive": CaseSensitive,
  };

  type Props = {
    name: IconName;
    size?: "xs" | "sm" | "md" | "lg";
    class?: string;
  };

  let { name, size = "sm", class: className = "" }: Props = $props();

  const sizePx: Record<string, number> = { xs: 12, sm: 14, md: 16, lg: 20 };
  let IconComp = $derived(iconMap[name]);
</script>

<IconComp size={sizePx[size]} class={className} />
