'use client'
import { useEffect } from 'react'

const loadedLanguages = new Set<string>()

const PRISM_LANGUAGE_MAP: Record<string, () => Promise<any>> = {

  css: () => import('prismjs/components/prism-css'),
  scss: () => import('prismjs/components/prism-scss'),
  sass: () => import('prismjs/components/prism-sass'),
  less: () => import('prismjs/components/prism-less'),

  java: () => import('prismjs/components/prism-java'),
  php: () => import('prismjs/components/prism-php'),
  ruby: () => import('prismjs/components/prism-ruby'),
  c: () => import('prismjs/components/prism-c'),
  csharp: () => import('prismjs/components/prism-csharp'),

  bash: () => import('prismjs/components/prism-bash'),
  powershell: () => import('prismjs/components/prism-powershell'),
  batch: () => import('prismjs/components/prism-batch'),

  toml: () => import('prismjs/components/prism-toml'),
  sql: () => import('prismjs/components/prism-sql'),

  rst: () => import('prismjs/components/prism-rest'),
  latex: () => import('prismjs/components/prism-latex'),

  dockerfile: () => import('prismjs/components/prism-docker'),

  r: () => import('prismjs/components/prism-r'),
  matlab: () => import('prismjs/components/prism-matlab'),
  perl: () => import('prismjs/components/prism-perl'),
  lua: () => import('prismjs/components/prism-lua'),
  vim: () => import('prismjs/components/prism-vim'),
  groovy: () => import('prismjs/components/prism-groovy'),
  ini: () => import('prismjs/components/prism-ini'),
  makefile: () => import('prismjs/components/prism-makefile')
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

    languageLoader().then(() => {
      loadedLanguages.add(language)
    }).catch(() => {
      //
    })
  }, [language])
}
