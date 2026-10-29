import js from '@eslint/js'
import prettier from 'eslint-config-prettier'
import pluginVue from 'eslint-plugin-vue'
import globals from 'globals'
import tseslint from 'typescript-eslint'

// Без типовых правил (`recommendedTypeChecked`): они требуют полного прогона tsc и шумят на
// wasm-объектах; типы и так проверяет `vue-tsc`.
export default tseslint.config(
  { ignores: ['dist', 'engine', 'node_modules'] },
  js.configs.recommended,
  tseslint.configs.recommended,
  pluginVue.configs['flat/recommended'],
  {
    files: ['**/*.vue'],
    languageOptions: { parserOptions: { parser: tseslint.parser } },
  },
  {
    files: ['src/**/*.{ts,vue}'],
    languageOptions: { globals: globals.browser },
  },
  {
    files: ['**/*.{ts,vue}'],
    rules: {
      // Неопределённые имена ловит TypeScript (`__APP_VERSION__` объявлен в env.d.ts).
      'no-undef': 'off',
      // Hud, Hotbar — осознанно короткие имена.
      'vue/multi-word-component-names': 'off',
    },
  },
  { files: ['scripts/**', 'vite.config.ts'], languageOptions: { globals: globals.node } },
  // Последним: отключает правила форматирования, которыми владеет Prettier.
  prettier,
)
