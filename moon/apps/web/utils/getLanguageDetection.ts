'use client'

export const LANGUAGE_MAP: Record<string, string> = {
  '.js': 'javascript',
  '.mjs': 'javascript',
  '.cjs': 'javascript',
  '.jsx': 'jsx',
  '.ts': 'typescript',
  '.tsx': 'tsx',
  '.d.ts': 'typescript',

  '.html': 'html',
  '.htm': 'html',
  '.css': 'css',
  '.scss': 'scss',
  '.sass': 'sass',
  '.less': 'less',
  '.xml': 'xml',
  '.svg': 'xml',

  '.py': 'python',
  '.pyw': 'python',
  '.java': 'java',
  '.kt': 'kotlin',
  '.kts': 'kotlin',
  '.php': 'php',
  '.rb': 'ruby',
  '.go': 'go',
  '.rs': 'rust',
  '.c': 'c',
  '.cpp': 'cpp',
  '.cc': 'cpp',
  '.cxx': 'cpp',
  '.h': 'c',
  '.hpp': 'cpp',
  '.cs': 'csharp',
  '.swift': 'swift',
  '.mk': 'makefile',
  '.make': 'makefile',
  '.mak': 'makefile',

  '.sh': 'bash',
  '.bash': 'bash',
  '.zsh': 'bash',
  '.fish': 'bash',
  '.ps1': 'powershell',
  '.bat': 'batch',
  '.cmd': 'batch',

  '.json': 'json',
  '.jsonc': 'json',
  '.yaml': 'yaml',
  '.yml': 'yaml',
  '.toml': 'toml',
  '.ini': 'ini',
  '.csv': 'csv',
  '.sql': 'sql',

  '.md': 'markdown',
  '.mdx': 'markdown',
  '.rst': 'rst',
  '.tex': 'latex',

  '.dockerfile': 'dockerfile',
  '.gitignore': 'bash',
  '.env': 'bash',
  '.env.example': 'bash',
  '.gitattributes': 'bash',
  '.bzl': 'python',
  '.bazelrc': 'ini',
  '.bazelignore': 'bash',
  '.bazelversion': 'text',
  '.bazelproject': 'yaml',

  '.r': 'r',
  '.m': 'matlab',
  '.pl': 'perl',
  '.pm': 'perl',
  '.lua': 'lua',
  '.vim': 'vim',
  '.vimrc': 'vim',
  '.dockerignore': 'bash',
  '.eslintrc': 'json',
  '.eslintrc.js': 'javascript',
  '.eslintrc.json': 'json',
  '.prettierrc': 'json',
  '.babelrc': 'json',
  '.editorconfig': 'ini',
  '.buckconfig': 'toml',
  '.buckversion': 'text',
  '.swcrc': 'json'
}

export const SPECIAL_FILE_MAP: Record<string, string> = {
  "dockerfile": 'dockerfile',
  "rakefile": 'ruby',
  "gemfile": 'ruby',
  "podfile": 'ruby',
  "fastfile": 'ruby',
  "cartfile": 'swift',
  'package.json': 'json',
  'tsconfig.json': 'json',
  'composer.json': 'json',
  'pom.xml': 'xml',
  'build.gradle': 'groovy',
  'settings.gradle': 'groovy',
  "buck": 'python',
  "build": 'python',
  "workspace": 'python',
  'workspace.bazel': 'python',
  'build.bazel': 'python',
  "makefile": 'makefile',
  'makefile.linux': 'makefile',
  'makefile.win': 'makefile',
  'makefile.mac': 'makefile'
}

export const COMPOUND_EXTENSIONS: Record<string, string> = {
  '.d.ts': 'typescript',
  '.spec.js': 'javascript',
  '.test.js': 'javascript',
  '.spec.ts': 'typescript',
  '.test.ts': 'typescript',
  '.spec.tsx': 'tsx',
  '.test.tsx': 'tsx',
  '.config.js': 'javascript',
  '.config.ts': 'typescript',
  '.config.tsx': 'tsx',
  '.config.json': 'json',
  '.config.yaml': 'yaml',
  '.config.yml': 'yaml',
  '.config.toml': 'toml',
  '.config.ini': 'ini',
  '.config.env': 'bash',
  '.config.sh': 'bash',
  '.config.bash': 'bash',
  '.config.zsh': 'bash',
  '.config.fish': 'bash',
  '.config.ps1': 'powershell',
  '.config.bat': 'batch',
  '.config.cmd': 'batch',
  '.config.gitignore': 'bash',
  '.config.env.example': 'bash',
  '.config.gitattributes': 'bash',
  '.config.eslintrc': 'json',
  '.config.eslintrc.js': 'javascript',
  '.config.eslintrc.json': 'json',
  '.config.prettierrc': 'json',
  '.config.babelrc': 'json',
  '.config.editorconfig': 'ini',
  '.config.buckconfig': 'toml',
  '.config.swcrc': 'json',
  'vitest.config.js': 'javascript',
  'vitest.config.mts': 'typescript',
  'vitest.config.ts': 'typescript',
  'vitest.config.tsx': 'tsx',
  'vitest.config.json': 'json',
  'vitest.config.yaml': 'yaml',
  'vitest.config.yml': 'yaml',
  'vitest.config.toml': 'toml'
}

export function getLangFromFileName(fileName: string): string {
  if (!fileName) return 'text'
  const lowerFileName = fileName.toLowerCase()
  const baseName = lowerFileName.split('/').pop() || ''

  if (SPECIAL_FILE_MAP[baseName]) {
    return SPECIAL_FILE_MAP[baseName]
  }

  for (const [extension, language] of Object.entries(COMPOUND_EXTENSIONS)) {
    if (lowerFileName.endsWith(extension)) {
      return language
    }
  }

  const lastPart = lowerFileName.match(/\.[^./\\]+$/)

  if (lastPart) {
    const extension = lastPart[0]

    return LANGUAGE_MAP[extension] ?? 'text'
  }

  return 'text'
}
