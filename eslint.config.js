import standard from 'eslint-config-standard'
import globals from 'globals'
import importPlugin from 'eslint-plugin-import'
import nPlugin from 'eslint-plugin-n'
import promisePlugin from 'eslint-plugin-promise'

export default [
  {
    languageOptions: {
      globals: {
        ...globals.browser,
      }
    },
    plugins: {
      import: importPlugin,
      n: nPlugin,
      promise: promisePlugin
    },
    rules: {
      ...standard.rules,
    }
  }
]
