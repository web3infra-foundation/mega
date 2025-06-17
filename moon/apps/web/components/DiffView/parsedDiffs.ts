const extensionToLangMap: Record<string, string> = {
  ".ts": "typescript",
  ".tsx": "typescriptreact",
  ".js": "javascript",
  ".jsx": "javascriptreact",
  ".json": "json",
  ".md": "markdown",
  ".py": "python",
  ".rs": "rust",
  ".cpp": "cpp",
  ".c": "c",
  ".h": "cpp",
  ".java": "java",
  ".go": "go",
  ".sh": "bash",
  ".yml": "yaml",
  ".yaml": "yaml",
  ".css": "css",
  ".scss": "scss",
  ".html": "html",
  ".vue": "vue",
  ".toml": "toml",
};

function getLangFromPath(path: string): string {
  const ext = path.match(/\.[^./\\]+$/)?.[0]?.toLowerCase();
  
  return ext ? extensionToLangMap[ext] ?? "plaintext" : "plaintext";
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

    if (getLangFromPath(path) === "plaintext") {
      return {
        path,
        lang: getLangFromPath(path),
        diff: block,
      };
    }

    let diffWithHeader = block;
    const plusMatch = block.match(/^\+\+\+ b\/([^\n\r]+)/m);

    if(!plusMatch){
      const hunkIndex = block.indexOf("@@");

      let prefix = `--- a/${path}\n+++ b/${path}\n`;
      
      diffWithHeader = hunkIndex >= 0
        ? block.slice(0, hunkIndex) + prefix + block.slice(hunkIndex)
        : prefix + block;

      if (!diffWithHeader.endsWith("\n")) {
        diffWithHeader += "\n";
      }
    }

    return {
      path,
      lang: getLangFromPath(path),
      diff: diffWithHeader,
    };
  });
}
