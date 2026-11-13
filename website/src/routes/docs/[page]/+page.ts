import { error } from '@sveltejs/kit';
import { sources } from '$lib/content';
import { docsPages } from '$lib/site';
import type { EntryGenerator, PageLoad } from './$types';

export const entries: EntryGenerator = () => docsPages.map((doc) => ({ page: doc.id }));

export const load: PageLoad = ({ params }) => {
    const source = sources[params.page];
    if (!source) error(404, 'Not found');
    return { page: params.page, source };
};
