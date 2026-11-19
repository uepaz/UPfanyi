import js from '@eslint/js'
import { defineConfig, globalIgnores } from 'eslint/config'
import prettier from 'eslint-config-prettier'
import vue from 'eslint-plugin-vue'
import globals from 'globals'
import ts from 'typescript-eslint'

export default defineConfig(
  globalIgnores([
    'dist/**',
    'node_modules/**',
    'src-tauri/**',
    '.agents/**',
    '.github/**',
    'target/**',
    '.task-artifacts/**',
  ]),
  js.configs.recommended,
  ts.configs.recommended,
  vue.configs['flat/recommended'],
  {
    files: ['src/**/*.{ts,vue}'],
    languageOptions: {
      globals: globals.browser,
      parserOptions: { parser: ts.parser },
    },
    rules: {
      'vue/multi-word-component-names': ['error', { ignores: ['Notice'] }],
    },
  },
  { files: ['*.{js,ts}'], languageOptions: { globals: globals.node } },
  prettier,
)
