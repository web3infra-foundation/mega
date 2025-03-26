/** @type {import("eslint").Linter.Config} */
module.exports = {
  root: true,
  extends: ['@gitmono/eslint-config/base.js', '@gitmono/eslint-config/next.js', 'plugin:storybook/recommended']
}
