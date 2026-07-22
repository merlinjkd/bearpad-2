<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorView, keymap, placeholder } from '@codemirror/view';
	import { EditorState, Compartment } from '@codemirror/state';
	import { defaultKeymap, history, historyKeymap, undo, redo } from '@codemirror/commands';
	import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
	import { syntaxHighlighting, defaultHighlightStyle } from '@codemirror/language';
	import { searchKeymap } from '@codemirror/search';
	import { autocompletion, completionKeymap } from '@codemirror/autocomplete';
	import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';

	interface EditorExposed {
		hasSelection: () => boolean;
		handleCut: () => Promise<void>;
		handleCopy: () => Promise<void>;
		handlePaste: () => Promise<void>;
		undo: () => void;
		redo: () => void;
		handleSelectAll: () => void;
		transformSelection: (type: 'lowercase' | 'uppercase' | 'propercase') => void;
		markSaved: () => void;
		isDirty: () => boolean;
		getContent: () => string;
		loadContent: (content: string) => void;
	}

	let {
		onReady,
		theme = 'dark',
		fontSize = 14,
		fontFamily = "'SF Mono', 'Fira Code', 'Cascadia Code', monospace",
	}: {
		onReady?: (ref: EditorExposed) => void;
		theme?: string;
		fontSize?: number;
		fontFamily?: string;
	} = $props();

	let container: HTMLDivElement;
	let view: EditorView;
	let dirty = false;

	const themeCompartment = new Compartment();
	const fontSizeCompartment = new Compartment();
	const fontFamilyCompartment = new Compartment();

	function computeTheme(themeName: string) {
		if (themeName === 'light') {
			return EditorView.theme({
				'&': { backgroundColor: '#ffffff', color: '#333333', height: '100vh' },
				'.cm-gutters': { backgroundColor: '#f5f5f5', color: '#999999', border: 'none' },
				'.cm-activeLineGutter': { backgroundColor: '#e8e8e8' },
				'.cm-activeLine': { backgroundColor: '#f0f0f044' },
				'.cm-cursor': { borderLeftColor: '#333333' },
				'.cm-selectionBackground': { backgroundColor: '#add6ff' },
				'.cm-focused .cm-selectionBackground': { backgroundColor: '#add6ff' },
				'.cm-matchingBracket': { backgroundColor: '#d4d4d4' },
			});
		}
		return EditorView.theme({
			'&': { backgroundColor: '#1e1e1e', color: '#d4d4d4', height: '100vh' },
			'.cm-gutters': { backgroundColor: '#252526', color: '#858585', border: 'none' },
			'.cm-activeLineGutter': { backgroundColor: '#2a2d2e' },
			'.cm-activeLine': { backgroundColor: '#2a2d2e44' },
			'.cm-cursor': { borderLeftColor: '#aeafad' },
			'.cm-selectionBackground': { backgroundColor: '#264f78' },
			'.cm-focused .cm-selectionBackground': { backgroundColor: '#264f78' },
			'.cm-matchingBracket': { backgroundColor: '#4b4b4b' },
		});
	}

	function computeFontSize(size: number) {
		return EditorView.theme({
			'&': { fontSize: `${size}px` },
		});
	}

	function computeFontFamily(family: string) {
		return EditorView.theme({
			'.cm-scroller': { fontFamily: family },
		});
	}

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
				themeCompartment.of(computeTheme(theme)),
				fontSizeCompartment.of(computeFontSize(fontSize)),
				fontFamilyCompartment.of(computeFontFamily(fontFamily)),
				EditorView.updateListener.of((update) => {
					if (update.docChanged) {
						dirty = true;
					}
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

	$effect(() => {
		if (!view) return;
		view.dispatch({
			effects: themeCompartment.reconfigure(computeTheme(theme)),
		});
	});

	$effect(() => {
		if (!view) return;
		view.dispatch({
			effects: fontSizeCompartment.reconfigure(computeFontSize(fontSize)),
		});
	});

	$effect(() => {
		if (!view) return;
		view.dispatch({
			effects: fontFamilyCompartment.reconfigure(computeFontFamily(fontFamily)),
		});
	});

	onMount(() => {
		createEditor();

		if (onReady) {
			onReady({
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
					undo(view);
				},

				redo: () => {
					redo(view);
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
					else if (type === 'propercase')
						newText = text.toLowerCase().replace(/\b\w/g, (c) => c.toUpperCase());
					replaceSelection(newText);
				},

				markSaved: () => {
					dirty = false;
				},

				isDirty: () => dirty,

				getContent: () => view.state.doc.toString(),

				loadContent: (content: string) => {
					view.dispatch({
						changes: { from: 0, to: view.state.doc.length, insert: content },
						selection: { anchor: 0 },
					});
					dirty = false;
				},
			});
		}
	});

	onDestroy(() => {
		view?.destroy();
	});
</script>

<div bind:this={container}></div>