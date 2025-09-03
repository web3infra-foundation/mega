import { DiffItem } from "@gitmono/types/generated"

const extensionToLangMap: Record<string, string> = {
  // Note that the key here is lowercase
  '.ts': 'typescript',
  '.tsx': 'tsx',
  '.js': 'javascript',
  '.jsx': 'jsx',
  '.json': 'json',
  '.md': 'markdown',
  '.py': 'python',
  '.rs': 'rust',
  '.cpp': 'cpp',
  '.c': 'c',
  '.h': 'cpp',
  '.java': 'java',
  '.go': 'go',
  '.sh': 'bash',
  '.yml': 'yaml',
  '.yaml': 'yaml',
  '.css': 'css',
  '.scss': 'scss',
  '.html': 'html',
  '.vue': 'vue',
  '.toml': 'toml',
  'dockerfile': 'dockerfile',
  '.dockerfile': 'dockerfile',
  'license-mit': 'plaintext',
  'buck': 'plaintext',
  '.gitignore': 'plaintext',
  '.env': 'plaintext',
  'license-third-party': 'plaintext',
  'license-apache': 'plaintext',
  'workspace': 'plaintext',
  '.buckroot': 'plaintext',
  '.buckconfig': 'plaintext'
}

function getLangFromPath(path: string): string {
  const extMatch = path.match(/\.[^./\\]+$/)

  if (extMatch) {
    return extensionToLangMap[extMatch[0].toLowerCase()] ?? 'binary'
  } else {
    const lastPart = path.split('/').pop()?.toLowerCase()

    if (lastPart) {
      return extensionToLangMap[lastPart] ?? 'binary'
    }
  }

  return 'binary'
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

    if (getLangFromPath(path) === 'binary') {
      return {
        path,
        lang: getLangFromPath(path),
        diff: block.data
      }
    }

    let diffWithHeader = block.data
    const headerRegex = /^(diff|index|---|\+\+\+|new file mode|@@)/;
    const hunkContent = block.data
      .split('\n')
      .filter(line => !headerRegex.test(line.trim()));

    const isEmptyHunk = hunkContent.every(line => line.trim() === '');

    if (!block.data.includes('@@') || isEmptyHunk) {
      diffWithHeader = 'EMPTY_DIFF_MARKER';
    }

    if (!diffWithHeader.endsWith('\n')) {
      diffWithHeader += '\n'
    }

    return {
      path: block.path,
      lang: getLangFromPath(path),
      diff: diffWithHeader
    }
  })
}
