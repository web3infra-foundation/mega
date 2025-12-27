import { getLangFromFileNameToDiff } from '@/utils/getLanguageDetection'

export interface DiffItem {
  data: string
  path: string
}

export function parsedDiffs(diffText: DiffItem[]): { path: string; lang: string; diff: string }[] {
  if (diffText.length < 1) return []

  return diffText.map((block) => {
    const lang = getLangFromFileNameToDiff(block.path)

    const isBinary = block.data.includes('Binary files differ')

    const isEmptyDiff = !block.data.includes('@@')

    let diff = block.data

    if (isEmptyDiff) {
      diff = 'EMPTY_DIFF_MARKER'
    }

    if (!diff.endsWith('\n')) {
      diff += '\n'
    }

    return {
      path: block.path,
      lang: isBinary ? 'binary' : lang,
      diff
    }
  })
}
