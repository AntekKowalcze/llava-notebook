import js from '@eslint/js';
import vueParser from 'vue-eslint-parser';
import pluginVue from 'eslint-plugin-vue';

export default [
  js.configs.recommended,
  {
    files: ['**/*.vue'],
    plugins: {
      vue: pluginVue,
    },
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
    rules: {
      'no-unused-vars': 'warn',
      'no-console': 'off',
      'vue/no-multiple-template-root': 'off',
    },
  },
  {
    files: ['**/*.{js,ts}'],
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
    },
  },
  {
    ignores: ['dist/', 'node_modules/', 'src-tauri/', '**/*.d.ts'],
  },
];
