import type { AnnotationSide, FileDiffMetadata } from '@pierre/diffs'

export function normalizeLine(content: string): string {
  return content
    .trim()
    .replace(/\/\/.*$/g, '')
    .replace(/\/\*.*?\*\//g, '')
    .trim()
}

export function extractLinesFromHunks(fileDiff: FileDiffMetadata, side: 'deletions' | 'additions'): string[] {
  const lines: string[] = []

  for (const hunk of fileDiff.hunks) {
    for (const content of hunk.hunkContent) {
      if (content.type === 'context') {
        lines.push(...content.lines)
      } else if (content.type === 'change') {
        if (side === 'deletions') {
          lines.push(...content.deletions)
        } else {
          lines.push(...content.additions)
        }
      }
    }
  }

  return lines
}

function getLineContextFromLines(
  lines: string[],
  startLineNumber: number,
  endLineNumber: number,
  contextSize: number
): {
  normalizedContent: string
  contextBefore: string[]
  contextAfter: string[]
  selectedLines: string[]
} | null {
  const startIndex = startLineNumber - 1
  const endIndex = endLineNumber - 1

  if (startIndex < 0 || endIndex >= lines.length || startIndex > endIndex) {
    return null
  }

  const selectedLines = lines.slice(startIndex, endIndex + 1)
  const normalizedContent = selectedLines.map((line) => normalizeLine(line)).join('\n')

  const contextBeforeStart = Math.max(0, startIndex - contextSize)
  const contextBefore = lines.slice(contextBeforeStart, startIndex).map((line) => normalizeLine(line))

  const contextAfterEnd = Math.min(lines.length, endIndex + contextSize + 1)
  const contextAfter = lines.slice(endIndex + 1, contextAfterEnd).map((line) => normalizeLine(line))

  return {
    selectedLines,
    normalizedContent,
    contextBefore,
    contextAfter
  }
}

export function getLineContext(
  fileDiff: FileDiffMetadata,
  side: AnnotationSide,
  startLineNumber: number,
  endLineNumber: number | null = null,
  contextSize: number = 3
): {
  normalizedContent: string
  contextBefore: string[]
  contextAfter: string[]
  selectedLines: string[]
} | null {
  const actualEndLineNumber = endLineNumber ?? startLineNumber
  const lines = extractLinesFromHunks(fileDiff, side)

  if (lines.length === 0) {
    return null
  }

  return getLineContextFromLines(lines, startLineNumber, actualEndLineNumber, contextSize)
}
