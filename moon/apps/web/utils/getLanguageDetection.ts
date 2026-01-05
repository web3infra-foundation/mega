'use client'

export const LANGUAGE_MAP: Record<string, string> = {
  '.js': 'javascript',
  '.mjs': 'javascript',
  '.cjs': 'javascript',
  '.jsx': 'jsx',
  '.ts': 'typescript',
  '.mts': 'typescript',
  '.cts': 'typescript',
  '.tsx': 'tsx',
  '.d.ts': 'typescript',
  '.vue': 'typescript',

  '.css': 'css',
  '.scss': 'scss',
  '.sass': 'sass',
  '.less': 'less',

  '.html': 'html',
  '.htm': 'html',
  '.xml': 'xml',
  '.iml': 'xml',
  '.svg': 'xml',

  '.c': 'c',
  '.h': 'c',
  '.cpp': 'cpp',
  '.cc': 'cpp',
  '.cxx': 'cpp',
  '.hpp': 'cpp',
  '.cs': 'csharp',
  '.go': 'go',
  '.rs': 'rust',
  '.swift': 'swift',
  '.kt': 'kotlin',
  '.kts': 'kotlin',
  '.java': 'java',
  '.d': 'd',
  '.zig': 'zig',
  '.nim': 'nim',
  '.v': 'v',

  '.py': 'python',
  '.pyw': 'python',
  '.rb': 'ruby',
  '.php': 'php',
  '.pl': 'perl',
  '.pm': 'perl',
  '.lua': 'lua',
  '.r': 'r',
  '.dart': 'dart',
  '.jl': 'julia',

  '.scala': 'scala',
  '.sbt': 'scala',
  '.clj': 'clojure',
  '.cljs': 'clojure',
  '.edn': 'clojure',
  '.ex': 'elixir',
  '.exs': 'elixir',
  '.erl': 'erlang',
  '.hrl': 'erlang',
  '.hs': 'haskell',
  '.lhs': 'haskell',
  '.ml': 'ocaml',
  '.mli': 'ocaml',
  '.fs': 'fsharp',
  '.fsx': 'fsharp',

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
  '.properties': 'properties',
  '.csv': 'csv',

  '.sql': 'sql',
  '.graphql': 'graphql',
  '.gql': 'graphql',

  '.md': 'markdown',
  '.mdx': 'markdown',
  '.txt': 'text',
  '.rst': 'rest',
  '.tex': 'latex',
  '.patch': 'diff',
  '.diff': 'diff',

  '.ejs': 'ejs',
  '.hbs': 'handlebars',
  '.handlebars': 'handlebars',
  '.mustache': 'handlebars',
  '.pug': 'pug',
  '.jade': 'pug',

  '.dockerfile': 'docker',
  '.nginx': 'nginx',
  '.htaccess': 'apacheconf',
  '.tf': 'hcl',
  '.tfvars': 'hcl',
  '.hcl': 'hcl',
  '.nix': 'nix',

  '.mk': 'makefile',
  '.make': 'makefile',
  '.mak': 'makefile',
  '.cmake': 'cmake',
  '.gradle': 'groovy',
  '.groovy': 'groovy',

  '.rego': 'rego',
  '.cedar': 'rego',
  '.polar': 'rego',
  '.sentinel': 'rego',

  '.sol': 'solidity',
  '.vy': 'python',

  '.asm': 'nasm',
  '.s': 'nasm',

  '.proto': 'protobuf',
  '.ipynb': 'json',
  '.webmanifest': 'json',

  '.m': 'matlab',

  '.vim': 'vim',
  '.vimrc': 'vim',
  '.editorconfig': 'ini',

  '.gitignore': 'bash',
  '.gitattributes': 'bash',
  '.dockerignore': 'bash',
  '.npmignore': 'bash',
  '.env': 'bash',
  '.env.example': 'bash',
  '.eslintrc': 'json',
  '.eslintrc.js': 'javascript',
  '.eslintrc.json': 'json',
  '.prettierrc': 'json',
  '.babelrc': 'json',
  '.swcrc': 'json',

  '.bzl': 'python',
  '.bazelrc': 'ini',
  '.bazelignore': 'bash',
  '.bazelversion': 'text',
  '.bazelproject': 'yaml',

  '.buckconfig': 'toml',
  '.buckversion': 'text',


  '.vtt': 'text',
  '.srt': 'text',
  '.log': 'text'
}

export const SPECIAL_FILE_MAP: Record<string, string> = {
  dockerfile: 'docker',

  rakefile: 'ruby',
  gemfile: 'ruby',
  podfile: 'ruby',
  fastfile: 'ruby',
  vagrantfile: 'ruby',
  brewfile: 'ruby',
  berksfile: 'ruby',
  guardfile: 'ruby',
  appraisals: 'ruby',
  dangerfile: 'ruby',

  cartfile: 'swift',

  'package.json': 'json',
  'tsconfig.json': 'json',
  'composer.json': 'json',

  'pom.xml': 'xml',

  'build.gradle': 'groovy',
  'settings.gradle': 'groovy',
  jenkinsfile: 'groovy',

  buck: 'python',
  build: 'python',
  workspace: 'python',
  'workspace.bazel': 'python',
  'build.bazel': 'python',

  makefile: 'makefile',
  'makefile.linux': 'makefile',
  'makefile.win': 'makefile',
  'makefile.mac': 'makefile',
  gnumakefile: 'makefile',
  justfile: 'bash',

  'cmakelists.txt': 'cmake',

  procfile: 'text',
  '.gitmodules': 'ini',
  '.yarnrc': 'ini',

  '.nvmrc': 'text',
  '.node-version': 'text',
  '.ruby-version': 'text',
  '.python-version': 'text',
  '.tool-versions': 'text',

  license: 'text',
  licence: 'text',
  authors: 'text',
  contributors: 'text',
  copying: 'text',
  notice: 'text',
  patents: 'text',
  changelog: 'markdown',
  history: 'markdown',
  news: 'markdown',
  releases: 'markdown',
  readme: 'markdown',
  todo: 'markdown'
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

export function getLangFromFileNameToDiff(fileName: string): string {
  if (!fileName) return 'plaintext'
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

    return LANGUAGE_MAP[extension] ?? 'plaintext'
  }

  return 'plaintext'
}
