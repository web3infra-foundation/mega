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
  '.buckconfig': 'plaintext',
}

function getLangFromPath(path: string): string {
  const extMatch = path.match(/\.[^./\\]+$/);
  
  if(extMatch) {
    return extensionToLangMap[extMatch[0].toLowerCase()] ?? "binary";
  } else {
    const lastPart = path.split('/').pop()?.toLowerCase();

    if(lastPart) {
      return extensionToLangMap[lastPart] ?? "binary";
    }
  }

  return "binary";
}

export function parsedDiffs(diffText: string): { path: string; lang: string; diff: string }[] {
  if (!diffText) return [];

  const parts = diffText
    .split(/(?=^diff --git )/gm)
    .map((block) => block.trim())
    .filter(Boolean);

  return parts.map((block) => {
    let path = "";

    const diffGitMatch = block.match(/^diff --git a\/[^\s]+ b\/([^\s]+)/m);

    if (diffGitMatch) {
      const originalPath = diffGitMatch[1]?.trim();
      const newPath = diffGitMatch[2]?.trim();

      if (newPath && newPath !== '/dev/null') {
        path = newPath;
      } else {
        path = originalPath;
      }
    }

    if (getLangFromPath(path) === "binary") {
      return {
        path,
        lang: getLangFromPath(path),
        diff: block,
      };
    }

    let diffWithHeader = block;
    const plusMatch = block.match(/^\+\+\+ b\/([^\n\r]+)/m);
    const hunkIndex = block.indexOf("@@");

    if(!plusMatch){
      let prefix = `--- a/${path}\n+++ b/${path}\n`;

      diffWithHeader = hunkIndex >= 0
        ? block.slice(0, hunkIndex) + prefix + block.slice(hunkIndex)
        : prefix + block;

    } else if(hunkIndex < 0){
      diffWithHeader = 'EMPTY_DIFF_MARKER'
    }

    if (!diffWithHeader.endsWith("\n")) {
      diffWithHeader += "\n";
    }

    return {
      path,
      lang: getLangFromPath(path),
      diff: diffWithHeader,
    };
  });
}
