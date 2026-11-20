import { defineConfig } from 'vitest/config';

const stress = 'tests/**/stress.test.ts';

export default defineConfig({
  test: {
    globals: true,
    pool: 'threads',
    // Vitest otherwise leaves one CPU idle, halving concurrency on a two-core CI runner.
    maxWorkers: process.env.CI ? '100%' : undefined,
    testTimeout: 60000,
    projects: [
      {
        test: {
          name: 'unit',
          include: ['tests/**/*.{test,spec}.{ts,js}'],
          exclude: [stress],
          sequence: { groupOrder: 0 },
        },
      },
      {
        test: {
          name: 'stress',
          include: [stress],
          maxWorkers: 2,
          sequence: { groupOrder: 1 },
        },
      },
    ],
  },
});
