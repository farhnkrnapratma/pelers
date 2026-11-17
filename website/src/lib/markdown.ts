import { Marked } from 'marked';
import hljs from 'highlight.js/lib/common';
import diagrams from './generated/diagrams.json';

const escapeHtml = (value: string): string =>
    value.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character] ?? character);

const languageAliases: Record<string, string> = { js: 'javascript', ts: 'typescript', sh: 'bash', shell: 'bash', svg: 'xml', html: 'xml' };

const copyButton = (code: string): string =>
    `<button type="button" class="doc-code-copy" data-copy-code="${escapeHtml(code)}" title="Copy code" aria-label="Copy code">Copy</button>`;

const marked = new Marked({
    gfm: true,
    renderer: {
        code({ text, lang }) {
            const language = languageAliases[(lang ?? '').trim()] ?? (lang ?? '').trim();
            if (language === 'mermaid' && text in diagrams) {
                return `<figure class="doc-diagram" data-diagram>${(diagrams as Record<string, string>)[text]}</figure>`;
            }
            const highlighted = language && hljs.getLanguage(language)
                ? hljs.highlight(text, { language }).value
                : escapeHtml(text);
            return `<figure class="doc-code"><figcaption><span>${escapeHtml(language || 'text')}</span>${copyButton(text)}</figcaption><pre><code class="hljs">${highlighted}</code></pre></figure>`;
        },
        heading({ tokens, depth }) {
            const text = this.parser.parseInline(tokens);
            const id = text.replace(/<[^>]+>/g, '').toLowerCase().trim().replace(/[^\w\s-]/g, '').replace(/\s+/g, '-');
            return `<h${depth} id="${id}"><a href="#${id}">${text}</a></h${depth}>\n`;
        },
    },
});

export const renderMarkdown = (source: string): string => marked.parse(source, { async: false }) as string;

export const stripFrontmatter = (source: string): { body: string; title: string; description: string } => {
    const match = source.match(/^---\n([\s\S]*?)\n---\n?/);
    const meta = match ? match[1] : '';
    const field = (name: string) => meta.match(new RegExp(`^${name}: (.*)$`, 'm'))?.[1]?.trim() ?? '';
    return { body: match ? source.slice(match[0].length) : source, title: field('title'), description: field('description') };
};
