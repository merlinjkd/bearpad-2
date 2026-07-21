<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorView, keymap, placeholder } from '@codemirror/view';
	import { EditorState, Compartment } from '@codemirror/state';
	import { defaultKeymap, history, historyKeymap, indentLess, indentMore } from '@codemirror/commands';
	import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
	import { syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
	import { searchKeymap } from '@codemirror/search';
	import { autocompletion, completionKeymap } from '@codemirror/autocomplete';
	import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';

	let { editorRef }: { editorRef?: any } = $props();

	let container: HTMLDivElement;
	let view: EditorView;

	function createEditor() {
		const state = EditorState.create({
			doc: '',
			extensions: [
				history(),
				keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, ...completionKeymap]),
				markdown({ base: markdownLanguage }),
				syntaxHighlighting(defaultHighlightStyle),
				autocompletion(),
				placeholder('Start typing...'),
				EditorView.theme({
					'&': { backgroundColor: '#1e1e1e', color: '#d4d4d4', height: '100vh' },
					'.cm-gutters': { backgroundColor: '#252526', color: '#858585', border: 'none' },
					'.cm-activeLineGutter': { backgroundColor: '#2a2d2e' },
					'.cm-activeLine': { backgroundColor: '#2a2d2e44' },
					'.cm-cursor': { borderLeftColor: '#aeafad' },
					'.cm-selectionBackground': { backgroundColor: '#264f78' },
					'.cm-focused .cm-selectionBackground': { backgroundColor: '#264f78' },
					'.cm-matchingBracket': { backgroundColor: '#4b4b4b' },
				}),
			],
		});

		view = new EditorView({
			state,
			parent: container,
		});
	}

	function getSelectedText(): string {
		const sel = view.state.selection.main;
		return sel.empty ? '' : view.state.sliceDoc(sel.from, sel.to);
	}

	function replaceSelection(text: string) {
		const sel = view.state.selection.main;
		if (sel.empty) return;
		view.dispatch({
			changes: { from: sel.from, to: sel.to, insert: text },
			selection: { anchor: sel.from + text.length },
		});
	}

	onMount(() => {
		createEditor();

		editorRef = {
			hasSelection: () => !view.state.selection.main.empty,

			handleCopy: async () => {
				const text = getSelectedText();
				if (!text) return;
				try {
					await writeText(text);
				} catch {
					await navigator.clipboard.writeText(text);
				}
			},

			handleCut: async () => {
				const text = getSelectedText();
				if (!text) return;
				try {
					await writeText(text);
				} catch {
					await navigator.clipboard.writeText(text);
				}
				const sel = view.state.selection.main;
				view.dispatch({
					changes: { from: sel.from, to: sel.to, insert: '' },
				});
			},

			handlePaste: async () => {
				let rawText = '';
				try {
					rawText = await navigator.clipboard.readText();
					if (!rawText) rawText = (await readText()) ?? '';
				} catch {
					rawText = (await readText()) ?? '';
				}
				if (!rawText) return;
				const sel = view.state.selection.main;
				view.dispatch({
					changes: { from: sel.from, to: sel.to, insert: rawText },
					selection: { anchor: sel.from + rawText.length },
				});
			},

			undo: () => {
				view.dispatch({ effects: history() });
			},

			redo: () => {
				view.dispatch({ effects: history() });
			},

			handleSelectAll: () => {
				view.dispatch({
					selection: { anchor: 0, head: view.state.doc.length },
				});
			},

			transformSelection: (type: 'lowercase' | 'uppercase' | 'propercase') => {
				const text = getSelectedText();
				if (!text) return;
				let newText = text;
				if (type === 'lowercase') newText = text.toLowerCase();
				else if (type === 'uppercase') newText = text.toUpperCase();
				else if (type === 'propercase') newText = text.toLowerCase().replace(/\b\w/g, (c) => c.toUpperCase());
				replaceSelection(newText);
			},
		};
	});

	onDestroy(() => {
		view?.destroy();
	});
</script>

<div bind:this={container}></div>
