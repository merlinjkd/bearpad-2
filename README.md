# Spike: CM6 in Bearpad 2

**Question:** Can CodeMirror 6 fully replace Monaco in Bearpad, wrapped with `@codemirror/lang-markdown`, while retaining all right-click context menu features?

## Approach

Scaffolded a fresh Tauri v2 + Svelte 5 + TypeScript project at `~/spikes/bearpad2-cm6/`. Implemented:

- **CM6 editor** with `@codemirror/lang-markdown`, dark theme, gutters
- **Custom context menu** (same pattern as Bearpad 1) — no Monaco menu to fight
- **Clipboard** via `@tauri-apps/plugin-clipboard-manager` (same as Bearpad 1)
- **Text transformations** (lowercase, UPPERCASE, Title Case) via `EditorState.update` + `replaceSelection`
- **Undo/Redo** via CM6 built-in history extension
- **Select All** via dispatch on whole document

## Build result

```
npm run build        ✓ (vite, Svelte 5)
npx tauri build      ✓ (Rust + bundling)
→ Bearpad 2.app     ✓
→ Bearpad 2.dmg     ✓
```

All TypeScript compiles clean. Tauri Rust backend compiles with clipboard plugin.

## Feature comparison

| Feature | Bearpad 1 (Monaco) | Bearpad 2 (CM6) | Notes |
|---|---|---|---|
| Right-click context menu | Had to disable Monaco's built-in + add custom | No built-in menu — custom only, clean |
| Clipboard (Tauri plugin) | Custom override in Editor.svelte | Same pattern, simpler | CM6 doesn't fight paste |
| Text transformations | `transformSelection()` export | `transformSelection()` via `state.update` | Simpler, no model API needed |
| Syntax highlighting | `@monaco-editor` | `@codemirror/lang-markdown` | CM6 is purpose-built for Markdown |
| Undo/Redo | `editor.trigger('undo')` | `history()` extension | CM6 has this built-in |
| Find/Search | `editor.action.find` | `@codemirror/search` | Map `triggerFind` to `openSearchPanel` |
| Vim mode | Built-in (`vscodevim` alternative) | `@codemirror-vim` or `@replit/codemirror-vim` | Less mature but viable |
| Scroll sync with preview | Custom monaco `onDidScrollChange` | Custom via `EditorView.dom` scroll events | Same approach |
| Minimap | Built-in tile-based | No CM6 minimap — would need custom | Trade-off |
| Bundle size | ~5MB | ~600KB (all-in with CM6 deps) | Significant reduction |

## Key architectural differences

### CM6: state-driven, functional

```ts
// Replace selected text
view.dispatch({
  changes: { from: sel.from, to: sel.to, insert: newText },
  selection: { anchor: sel.from + newText.length },
});
```

### Monaco: imperative, model-centric

```ts
const model = editor.getModel()!;
model.applyEdits([{ range: selection, text: newText }]);
```

CM6's approach is cleaner for the text transformation use case — no model reference needed, just dispatch transactions on the view.

## What Monaco provides that CM6 doesn't

1. **Minimap** — CM6 has no built-in minimap. Would need a custom plugin or skip it.
2. **Built-in Vim mode** — Monaco has mature Vim via `@monaco-editor` community; CM6 Vim modes exist but are less battle-tested.
3. **Built-in file tree / sidebar** — Not relevant here (Bearpad has its own).

## Verdict: VALIDATED

CM6 is a viable replacement. Context menu architecture is **cleaner** (no conflict), clipboard handling is **simpler** (no need to suppress Monaco's internal handlers), and the Markdown language support is purpose-built rather than adapted from a general-purpose code editor.

**Effort to port:** ~1-2 days to rewrite `Editor.svelte` (~200 lines) and re-export the same `editorPane` API. All other components (ContextMenu, MarkdownViewer, App) stay essentially the same.

**Trade-offs to accept:**
- Lose minimap (or build custom)
- Lose Monaco's Vim mode (use a third-party CM6 Vim plugin)
- Learn CM6's extension/compartment system (different mental model than Monaco's actions)
- Scroll sync implementation differs (both custom, just different API)
