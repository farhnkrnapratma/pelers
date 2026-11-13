import home from './content/home.md?raw';
import api from './content/docs/api.md?raw';
import compatibility from './content/docs/compatibility.md?raw';
import security from './content/docs/security.md?raw';
import theming from './content/docs/theming.md?raw';

export const sources: Record<string, string> = { home, api, compatibility, security, theming };
