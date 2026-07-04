<script lang="ts">
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { HighlightStyle, StreamLanguage, syntaxHighlighting } from "@codemirror/language";
  import { shell } from "@codemirror/legacy-modes/mode/shell";
  import { EditorState } from "@codemirror/state";
  import { tags } from "@lezer/highlight";
  import {
    drawSelection,
    EditorView,
    highlightSpecialChars,
    keymap,
    lineNumbers,
    placeholder as editorPlaceholder,
  } from "@codemirror/view";
  import { onDestroy, onMount } from "svelte";

  type Props = {
    label: string;
    value?: string;
    placeholder?: string;
    error?: string | null;
    onblur?: () => void;
  };

  let {
    label,
    value = $bindable(""),
    placeholder = "",
    error = null,
    onblur,
  }: Props = $props();

  let host: HTMLDivElement;
  let view = $state.raw<EditorView | null>(null);
  const labelId = `command-editor-${crypto.randomUUID()}`;
  const errorId = `${labelId}-error`;
  const commandTheme = EditorView.theme(
    {
      "&": {
        backgroundColor: "var(--color-surface)",
        color: "var(--color-text)",
      },
      ".cm-scroller": {
        fontFamily: "var(--font-mono)",
        lineHeight: "1.45",
      },
      ".cm-content": {
        caretColor: "var(--color-accent)",
      },
      ".cm-cursor, .cm-dropCursor": {
        borderLeftColor: "var(--color-accent)",
      },
      ".cm-gutters": {
        backgroundColor: "var(--color-canvas)",
        color: "var(--color-text-subtle)",
        borderRight: "1px solid var(--color-border)",
      },
      ".cm-activeLineGutter": {
        color: "var(--color-text-muted)",
      },
      ".cm-activeLine": {
        backgroundColor: "var(--color-surface-hover)",
      },
      ".cm-selectionBackground, ::selection": {
        backgroundColor: "var(--color-accent-soft)",
      },
      ".cm-focused .cm-selectionBackground": {
        backgroundColor: "var(--color-accent-soft)",
      },
    },
    {},
  );
  const commandHighlightStyle = HighlightStyle.define([
    { tag: tags.keyword, color: "var(--color-accent)" },
    { tag: tags.string, color: "var(--color-success)" },
    { tag: tags.number, color: "var(--color-warning)" },
    { tag: tags.comment, color: "var(--color-text-subtle)", fontStyle: "italic" },
    { tag: tags.operator, color: "var(--color-text-muted)" },
    { tag: tags.punctuation, color: "var(--color-text-muted)" },
    { tag: tags.variableName, color: "var(--color-text)" },
    { tag: tags.bool, color: "var(--color-danger)" },
  ]);

  function createState(doc: string) {
    return EditorState.create({
      doc,
      extensions: [
        highlightSpecialChars(),
        history(),
        drawSelection(),
        lineNumbers(),
        StreamLanguage.define(shell),
        syntaxHighlighting(commandHighlightStyle),
        commandTheme,
        editorPlaceholder(placeholder),
        EditorView.lineWrapping,
        keymap.of([...defaultKeymap, ...historyKeymap]),
        EditorView.contentAttributes.of({
          "aria-describedby": error ? errorId : "",
          "aria-invalid": error ? "true" : "false",
          "aria-labelledby": labelId,
          "aria-multiline": "true",
          role: "textbox",
        }),
        EditorView.domEventHandlers({
          blur: () => onblur?.(),
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            value = update.state.doc.toString();
          }
        }),
      ],
    });
  }

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: createState(value),
    });
  });

  onDestroy(() => {
    view?.destroy();
    view = null;
  });

  $effect(() => {
    const current = view?.state.doc.toString();
    if (!view || current === undefined || value === current) {
      return;
    }

    view.dispatch({
      changes: { from: 0, to: current.length, insert: value },
    });
  });
</script>

<div class={`command-code-editor grid gap-1.5 text-[12px] ${error ? "command-code-editor--error" : ""}`}>
  <span id={labelId} class="text-[12px] text-text-subtle">{label}</span>
  <div bind:this={host}></div>
  {#if error}
    <span id={errorId} class="text-xs text-danger">{error}</span>
  {/if}
</div>

<style>
  :global(.command-code-editor .cm-editor) {
    border: 1px solid var(--color-border);
    border-radius: 0.375rem;
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 13px;
    transition:
      border-color 75ms,
      background-color 75ms;
  }

  :global(.command-code-editor .cm-editor.cm-focused) {
    border-color: var(--color-accent);
    outline: none;
  }

  :global(.command-code-editor.command-code-editor--error .cm-editor) {
    border-color: var(--color-danger);
  }

  :global(.command-code-editor .cm-scroller) {
    min-height: 2.25rem;
    overflow: auto;
  }

  :global(.command-code-editor .cm-content) {
    padding: 0.45rem 0.5rem;
  }

  :global(.command-code-editor .cm-line) {
    padding: 0;
  }

  :global(.command-code-editor .cm-gutterElement) {
    min-width: 1.5rem;
    padding: 0.45rem 0.35rem 0.45rem 0.5rem;
  }

  :global(.command-code-editor .cm-placeholder) {
    color: var(--color-text-subtle);
  }

  :global(.command-code-editor .cm-selectionBackground),
  :global(.command-code-editor .cm-content ::selection) {
    background: var(--color-accent-soft);
  }
</style>
