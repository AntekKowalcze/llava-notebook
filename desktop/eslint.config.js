import js from '@eslint/js'
import vueParser from 'vue-eslint-parser'

export default [
    js.configs.recommended,
    {
        files: ['**/*.vue'],
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
]
