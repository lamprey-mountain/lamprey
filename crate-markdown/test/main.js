/**
 * Lamprey Markdown WASM Test Page
 * 
 * This script loads the WASM module and provides:
 * 1. One-shot parsing for SolidJS rendering (events)
 * 2. Token extraction for ProseMirror syntax highlighting
 * 3. Incremental editing test
 */

import init, { parse_markdown, render_markdown, render_plaintext, WasmParsed } from '../pkg/lamprey_markdown.js';

// DOM Elements
const statusEl = document.getElementById('status');
const inputEl = document.getElementById('markdown-input');
const renderedOutputEl = document.getElementById('rendered-output');
const eventsOutputEl = document.getElementById('events-output');
const tokensOutputEl = document.getElementById('tokens-output');
const plaintextOutputEl = document.getElementById('plaintext-output');
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
        
        // Perform initial parse
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
    
    // 1. Parse and get events + tokens (for SolidJS rendering)
    const parseResult = JSON.parse(parse_markdown(markdown));
    
    // Display events
    eventsOutputEl.textContent = formatEvents(parseResult.events);
    
    // Display tokens
    tokensOutputEl.innerHTML = formatTokens(parseResult.tokens);
    
    // 2. Render as markdown (identity)
    renderedOutputEl.innerHTML = renderMarkdownToHtml(render_markdown(markdown));
    
    // 3. Render as plain text
    plaintextOutputEl.textContent = render_plaintext(markdown);
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
                return `   Text: "${e.content}"`;
            case 'code':
                return `   Code: "${e.content}"`;
            case 'soft_break':
                return `   SoftBreak`;
            case 'hard_break':
                return `   HardBreak`;
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
 * Simple markdown to HTML conversion for display purposes.
 * In a real app, you'd use a proper renderer or SolidJS components.
 */
function renderMarkdownToHtml(md) {
    // This is a naive conversion for demonstration
    // In production, you'd use the events to build proper HTML
    let html = escapeHtml(md);
    
    // Basic formatting for display
    html = html
        .replace(/^### (.+)$/gm, '<h3>$1</h3>')
        .replace(/^## (.+)$/gm, '<h2>$1</h2>')
        .replace(/^# (.+)$/gm, '<h1>$1</h1>')
        .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
        .replace(/\*(.+?)\*/g, '<em>$1</em>')
        .replace(/~~(.+?)~~/g, '<del>$1</del>')
        .replace(/`([^`]+)`/g, '<code>$1</code>')
        .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
        .replace(/^> (.+)$/gm, '<blockquote>$1</blockquote>')
        .replace(/^- \[x\]/gm, '☑')
        .replace(/^- \[ \]/gm, '☐')
        .replace(/^- (.+)/gm, '<li>$1</li>')
        .replace(/(<li>.*<\/li>)/s, '<ul>$1</ul>');
    
    // Wrap paragraphs
    html = html.split('\n\n').map(p => {
        if (!p.startsWith('<')) {
            return `<p>${p.replace(/\n/g, '<br>')}</p>`;
        }
        return p;
    }).join('\n');
    
    return html;
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
    
    const steps = [
        { deleteStart: 2, deleteEnd: 7, insert: 'Greetings', desc: 'Change "Hello" to "Greetings"' },
        { deleteStart: 22, deleteEnd: 26, insert: 'demo', desc: 'Change "test" to "demo"' },
        { deleteStart: 0, deleteEnd: 0, insert: '> ', desc: 'Add blockquote marker' },
        { deleteStart: 30, deleteEnd: 30, insert: '\n\n**New paragraph**', desc: 'Add bold paragraph' },
    ];
    
    const results = [`Initial: "${initialText}"\nTokens count: ${JSON.parse(wasmParsed.tokens).length}`];
    
    for (const step of steps) {
        const tokensJson = wasmParsed.edit_and_tokens(
            step.deleteStart,
            step.deleteEnd,
            step.insert
        );
        const tokens = JSON.parse(tokensJson);
        
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
