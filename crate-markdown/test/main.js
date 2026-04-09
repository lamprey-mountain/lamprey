/**
 * Lamprey Markdown WASM Test Page
 *
 * This script loads the WASM module and demonstrates:
 * 1. One-shot parsing with entity extraction (mentions, emoji, spoilers)
 * 2. Token extraction for ProseMirror syntax highlighting
 * 3. Incremental editing with tree reuse
 */

import init, { parse_markdown, render_markdown, render_plaintext, WasmParsed } from '../pkg/lamprey_markdown.js';

// DOM Elements
const statusEl = document.getElementById('status');
const inputEl = document.getElementById('markdown-input');
const renderedOutputEl = document.getElementById('rendered-output');
const eventsOutputEl = document.getElementById('events-output');
const tokensOutputEl = document.getElementById('tokens-output');
const plaintextOutputEl = document.getElementById('plaintext-output');
const entitiesOutputEl = document.getElementById('entities-output');
const incrementalBtn = document.getElementById('incremental-btn');
const incrementalStatusEl = document.getElementById('incremental-status');
const incrementalOutputEl = document.getElementById('incremental-output');

// State
let wasmLoaded = false;
let wasmParsed = null;

/**
 * Initialize the WASM module
 */
async function initWasm() {
    try {
        await init();
        wasmLoaded = true;
        statusEl.textContent = 'WASM module loaded successfully';
        statusEl.style.color = '#a6e3a1';
        updateOutput();
    } catch (error) {
        statusEl.textContent = `Failed to load WASM: ${error.message}`;
        statusEl.style.color = '#f38ba8';
        console.error('WASM initialization failed:', error);
    }
}

/**
 * Update all output panels based on current input
 */
function updateOutput() {
    if (!wasmLoaded) return;

    const markdown = inputEl.value;

    // 1. Parse — returns native JS object (no JSON.parse needed!)
    const result = parse_markdown(markdown);

    // Display events
    const events = Array.isArray(result.events) ? result.events : [];
    eventsOutputEl.textContent = formatEvents(events);

    // Display tokens
    const tokens = Array.isArray(result.tokens) ? result.tokens : [];
    tokensOutputEl.innerHTML = formatTokens(tokens);

    // Display extracted entities
    const mentions = Array.isArray(result.mentions) ? result.mentions : [];
    const emoji = Array.isArray(result.emoji) ? result.emoji : [];
    const spoilers = Array.isArray(result.spoilers) ? result.spoilers : [];
    entitiesOutputEl.textContent = formatEntities(mentions, emoji, spoilers);

    // 2. Render as plain text
    plaintextOutputEl.textContent = render_plaintext(markdown);

    // 3. Render as markdown (identity — returns the input back)
    renderedOutputEl.textContent = render_markdown(markdown);
}

/**
 * Format events for display
 */
function formatEvents(events) {
    return events.map(e => {
        switch (e.type) {
            case 'start':
                return `▶ Start: ${e.tag}`;
            case 'end':
                return `◀ End: ${e.tag}`;
            case 'text':
                return `   Text: "${e.content.replace(/\n/g, '\\n')}"`;
            case 'code':
                return `   Code: "${e.content}"`;
            case 'rule':
                return `   Rule`;
            case 'html':
                return `   Html: "${e.content}"`;
            default:
                return `   Unknown: ${JSON.stringify(e)}`;
        }
    }).join('\n');
}

/**
 * Format tokens with syntax highlighting spans
 */
function formatTokens(tokens) {
    return tokens.map(t => {
        const className = `token token-${t.kind.toLowerCase().replace('_', '-')}`;
        return `<span class="${className}" title="Start: ${t.start}, End: ${t.end}">${escapeHtml(t.text)}</span>`;
    }).join('');
}

/**
 * Format extracted entities for display
 */
function formatEntities(mentions, emoji, spoilers) {
    const lines = [];

    if (mentions.length > 0) {
        lines.push(`Mentions (${mentions.length}):`);
        for (const m of mentions) {
            lines.push(`  - ${m.mention_type}: ${m.id}`);
        }
    }

    if (emoji.length > 0) {
        lines.push(`Emoji (${emoji.length}):`);
        for (const e of emoji) {
            lines.push(`  - :${e.name}: ${e.id}${e.animated ? ' (animated)' : ''}`);
        }
    }

    if (spoilers.length > 0) {
        lines.push(`Spoilers (${spoilers.length}):`);
        for (const s of spoilers) {
            lines.push(`  - content range: ${s.content_start}..${s.content_end}`);
        }
    }

    if (lines.length === 0) {
        return '(no entities found)';
    }

    return lines.join('\n');
}

/**
 * Escape HTML special characters
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Test incremental editing
 */
async function testIncrementalEdit() {
    if (!wasmLoaded) return;

    incrementalBtn.disabled = true;
    incrementalStatusEl.textContent = 'Testing...';

    const initialText = '# Hello World\n\nThis is a test.';
    wasmParsed = new WasmParsed(initialText);

    // tokens is already a JS array, no JSON.parse needed
    const initialTokens = wasmParsed.tokens;

    const steps = [
        { deleteStart: 2, deleteEnd: 7, insert: 'Greetings', desc: 'Change "Hello" to "Greetings"' },
        { deleteStart: 22, deleteEnd: 26, insert: 'demo', desc: 'Change "test" to "demo"' },
        { deleteStart: 0, deleteEnd: 0, insert: '> ', desc: 'Add blockquote markers' },
        { deleteStart: 30, deleteEnd: 30, insert: '\n\n**New paragraph**', desc: 'Add bold paragraph' },
    ];

    const results = [
        `Initial: "${initialText}"\nTokens: ${initialTokens.length}`,
    ];

    for (const step of steps) {
        // edit_and_tokens returns a JS array directly
        const tokens = wasmParsed.edit_and_tokens(
            step.deleteStart,
            step.deleteEnd,
            step.insert
        );

        results.push(
            `\n${step.desc}\n` +
            `Source: "${wasmParsed.source.substring(0, 50)}..."\n` +
            `Tokens: ${tokens.length}`
        );
    }

    incrementalOutputEl.textContent = results.join('\n---\n');
    incrementalStatusEl.textContent = `Completed ${steps.length} edits`;
    incrementalBtn.disabled = false;
}

// Event listeners
inputEl.addEventListener('input', updateOutput);
incrementalBtn.addEventListener('click', testIncrementalEdit);

// Initialize
initWasm();
