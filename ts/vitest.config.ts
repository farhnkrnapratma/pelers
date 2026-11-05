import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    pool: 'threads',
    // Vitest otherwise leaves one CPU idle, halving concurrency on a two-core CI runner.
    maxWorkers: process.env.CI ? '100%' : undefined,
    testTimeout: 60000,
    include: ['tests/**/*.{test,spec}.{ts,js}'],
  },
});
