import { parsePatchFiles, type FileDiffMetadata } from '@pierre/diffs'

import { getLanguageForFile } from '@/utils/shikiLanguageFallback'

export interface DiffItem {
  data: string
  path: string
}

export interface ParsedFile {
  file: { path: string; lang: string; diff: string }
  fileDiffMetadata: FileDiffMetadata | null
  stats: { additions: number; deletions: number }
}

export function parsedDiffs(diffText: DiffItem[]): { path: string; lang: string; diff: string }[] {
  if (diffText.length < 1) return []

  return diffText.map((block) => {
    const isBinary = /Binary files.*differ/.test(block.data)

    let diff = block.data

    if (isBinary) {
      /* empty */
    } else {
      const isEmptyDiff = !block.data.includes('@@')

      if (isEmptyDiff) {
        diff = 'EMPTY_DIFF_MARKER'
      }
    }

    if (!diff.endsWith('\n')) {
      diff += '\n'
    }

    return {
      path: block.path,
      lang: isBinary ? 'binary' : 'auto',
      diff
    }
  })
}

export function generateParsedFiles(diffFiles: { path: string; lang: string; diff: string }[]): ParsedFile[] {
  return diffFiles.map((file) => {
    if (file.lang === 'binary' || file.diff === 'EMPTY_DIFF_MARKER\n') {
      return {
        file,
        fileDiffMetadata: null,
        stats: { additions: 0, deletions: 0 }
      }
    }

    let fileDiffMetadata: FileDiffMetadata | null = null
    let additions = 0
    let deletions = 0

    try {
      const patches = parsePatchFiles(file.diff)

      if (patches.length > 0 && patches[0].files.length > 0) {
        fileDiffMetadata = patches[0].files[0]

        if (!fileDiffMetadata.name && file.path) {
          fileDiffMetadata = { ...fileDiffMetadata, name: file.path }
        }

        const safeLang = getLanguageForFile(file.path)

        fileDiffMetadata = { ...fileDiffMetadata, lang: safeLang as any }
      }

      if (fileDiffMetadata) {
        for (const hunk of fileDiffMetadata.hunks) {
          additions += hunk.additionCount
          deletions += hunk.deletionCount
        }
      }
    } catch (e) {
      /* eslint-disable-next-line no-console */
      console.error('error parsing diff:', e)
    }

    return { file, fileDiffMetadata, stats: { additions, deletions } }
  })
}
