'use client'

import { useEffect } from 'react'

const loadedLanguages = new Set<string>()

/**
 * prism-react-renderer Default built-in languages (no dynamic loading required) :
 * markup (html, xml, svg), jsx, tsx, swift, kotlin, objectivec,
 * js-extras, reason, rust, graphql, yaml, go, cpp, markdown, python, json
 */

const PRISM_LANGUAGE_MAP: Record<string, () => Promise<any>> = {
  css: () => import('prismjs/components/prism-css'),
  scss: () => import('prismjs/components/prism-scss'),
  sass: () => import('prismjs/components/prism-sass'),
  less: () => import('prismjs/components/prism-less'),

  c: () => import('prismjs/components/prism-c'),
  csharp: () => import('prismjs/components/prism-csharp'),
  java: () => import('prismjs/components/prism-java'),
  typescript: () => import('prismjs/components/prism-typescript'),
  d: () => import('prismjs/components/prism-d'),
  zig: () => import('prismjs/components/prism-zig'),
  nim: () => import('prismjs/components/prism-nim'),
  v: () => import('prismjs/components/prism-v'),

  ruby: () => import('prismjs/components/prism-ruby'),
  php: () => import('prismjs/components/prism-php'),
  perl: () => import('prismjs/components/prism-perl'),
  lua: () => import('prismjs/components/prism-lua'),
  r: () => import('prismjs/components/prism-r'),
  dart: () => import('prismjs/components/prism-dart'),
  julia: () => import('prismjs/components/prism-julia'),

  scala: () => import('prismjs/components/prism-scala'),
  clojure: () => import('prismjs/components/prism-clojure'),
  elixir: () => import('prismjs/components/prism-elixir'),
  erlang: () => import('prismjs/components/prism-erlang'),
  haskell: () => import('prismjs/components/prism-haskell'),
  ocaml: () => import('prismjs/components/prism-ocaml'),
  fsharp: () => import('prismjs/components/prism-fsharp'),

  bash: () => import('prismjs/components/prism-bash'),
  powershell: () => import('prismjs/components/prism-powershell'),
  batch: () => import('prismjs/components/prism-batch'),

  toml: () => import('prismjs/components/prism-toml'),
  ini: () => import('prismjs/components/prism-ini'),
  properties: () => import('prismjs/components/prism-properties'),
  csv: () => import('prismjs/components/prism-csv'),

  sql: () => import('prismjs/components/prism-sql'),

  rest: () => import('prismjs/components/prism-rest'),
  latex: () => import('prismjs/components/prism-latex'),
  diff: () => import('prismjs/components/prism-diff'),

  ejs: () => import('prismjs/components/prism-ejs'),
  handlebars: () => import('prismjs/components/prism-handlebars'),
  pug: () => import('prismjs/components/prism-pug'),

  docker: () => import('prismjs/components/prism-docker'),
  nginx: () => import('prismjs/components/prism-nginx'),
  apacheconf: () => import('prismjs/components/prism-apacheconf'),
  hcl: () => import('prismjs/components/prism-hcl'),
  nix: () => import('prismjs/components/prism-nix'),

  makefile: () => import('prismjs/components/prism-makefile'),
  cmake: () => import('prismjs/components/prism-cmake'),
  groovy: () => import('prismjs/components/prism-groovy'),

  rego: () => import('prismjs/components/prism-rego'),

  solidity: () => import('prismjs/components/prism-solidity'),

  nasm: () => import('prismjs/components/prism-nasm'),

  protobuf: () => import('prismjs/components/prism-protobuf'),

  matlab: () => import('prismjs/components/prism-matlab'),

  vim: () => import('prismjs/components/prism-vim')
}

export function usePrismLanguageLoader(language: string) {
  useEffect(() => {
    if (language === 'text' || loadedLanguages.has(language)) {
      return
    }

    const languageLoader = PRISM_LANGUAGE_MAP[language]

    if (!languageLoader) {
      return
    }

    languageLoader()
      .then(() => {
        loadedLanguages.add(language)
      })
      .catch(() => {
        //
      })
  }, [language])
}
