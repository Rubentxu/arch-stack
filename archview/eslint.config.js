import tseslint from 'typescript-eslint';
import solid from 'eslint-plugin-solid';
import prettierConfig from 'eslint-config-prettier';

export default tseslint.config(
  // TypeScript recommended rules
  ...tseslint.configs.recommended,
  // SolidJS recommended rules
  {
    plugins: { solid: solid },
    rules: solid.configs.recommended.rules,
  },
  // Prettier last (disables conflicting ESLint style rules)
  prettierConfig,
  // Ignore patterns
  {
    ignores: ['dist/', 'node_modules/', '.vite/', 'coverage/'],
  }
);
