import { getLangFromFileNameToDiff } from '@/utils/getLanguageDetection'

export interface DiffItem {
  data: string
  path: string
}

export function parsedDiffs(diffText: DiffItem[]): { path: string; lang: string; diff: string }[] {
  if (diffText.length < 1) return []

  return diffText.map((block) => {
    let path = ''

    const diffGitMatch = block.data.match(/^diff --git a\/[^\s]+ b\/([^\s]+)/m)

    if (diffGitMatch) {
      const originalPath = diffGitMatch[1]?.trim()
      const newPath = diffGitMatch[2]?.trim()

      if (newPath && newPath !== '/dev/null') {
        path = newPath
      } else {
        path = originalPath
      }
    }

    if (getLangFromFileNameToDiff(path) === 'binary') {
      return {
        path,
        lang: getLangFromFileNameToDiff(path),
        diff: block.data
      }
    }

    let diffWithHeader = block.data
    const headerRegex = /^(diff|index|---|\+\+\+|new file mode|@@)/
    const hunkContent = block.data.split('\n').filter((line) => !headerRegex.test(line.trim()))

    const isEmptyHunk = hunkContent.every((line) => line.trim() === '')

    if (!block.data.includes('@@') || isEmptyHunk) {
      diffWithHeader = 'EMPTY_DIFF_MARKER'
    }

    if (!diffWithHeader.endsWith('\n')) {
      diffWithHeader += '\n'
    }

    return {
      path: block.path,
      lang: getLangFromFileNameToDiff(path),
      diff: diffWithHeader
    }
  })
}
