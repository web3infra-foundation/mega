import { parsePatchFiles, type ChangeTypes, type FileDiffMetadata } from '@pierre/diffs'

import { getLanguageForFile } from '@/utils/shikiLanguageFallback'

export interface DiffItem {
  data: string
  path: string
}

export interface ParsedFile {
  file: { path: string; lang: string; diff: string }
  fileDiffMetadata: FileDiffMetadata | null
  stats: { additions: number; deletions: number }
  changeType: ChangeTypes | null
  isBinary: boolean
  hasContent: boolean
}

export function parsedDiffs(diffText: DiffItem[]): { path: string; lang: string; diff: string }[] {
  if (diffText.length < 1) return []

  return diffText.map((block) => {
    const isBinary = /Binary files.*differ/.test(block.data)

    return {
      path: block.path,
      lang: isBinary ? 'binary' : 'auto',
      diff: block.data
    }
  })
}

export function generateParsedFiles(diffFiles: { path: string; lang: string; diff: string }[]): ParsedFile[] {
  return diffFiles.map((file) => {
    let fileDiffMetadata: FileDiffMetadata | null = null
    let additions = 0
    let deletions = 0
    let changeType: ChangeTypes | null = null
    const isBinary = file.lang === 'binary'
    let hasContent = false

    try {
      const patches = parsePatchFiles(file.diff)

      if (patches.length > 0 && patches[0].files.length > 0) {
        const parsed = patches[0].files[0]

        changeType = parsed.type || null
        hasContent = parsed.hunks.length > 0

        const safeLang = getLanguageForFile(file.path)

        fileDiffMetadata = {
          ...parsed,
          name: parsed.name || file.path,
          lang: safeLang as any
        }
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

    return { file, fileDiffMetadata, stats: { additions, deletions }, changeType, isBinary, hasContent }
  })
}
