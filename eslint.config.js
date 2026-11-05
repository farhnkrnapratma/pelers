import js from '@eslint/js';
import { defineConfig, globalIgnores } from 'eslint/config';
import tseslint from 'typescript-eslint';

export default defineConfig([
  globalIgnores([
    'node_modules/',
    'dist/',
    'target/',
    'graphify-out/',
    '.claude/',
    '.agents/',
    '.copilot/',
    'tests/compat/',
    'tests/corpus/',
    'tests/scratch/',
    'tests/support/usecase-reference.mjs',
  ]),
  js.configs.recommended,
  tseslint.configs.recommended,
  { files: ['**/*.{js,mjs}'], rules: { 'no-undef': 'off' } },
  {
    rules: {
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_', ignoreRestSiblings: true },
      ],
      '@typescript-eslint/no-explicit-any': 'warn',
      'no-empty': ['error', { allowEmptyCatch: true }],
      'no-useless-assignment': 'warn',
      'preserve-caught-error': 'off',
      'no-control-regex': 'off',
      'no-useless-escape': 'off',
      'no-regex-spaces': 'off',
      'no-irregular-whitespace': 'off',
    },
  },
  {
    files: ['src/layout/layered.ts'],
    rules: { '@typescript-eslint/no-unused-expressions': 'off', 'no-useless-assignment': 'off' },
  },
]);
