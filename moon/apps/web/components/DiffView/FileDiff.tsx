import { useEffect, useMemo, useRef, useState } from 'react'
import { DiffFile, DiffModeEnum, DiffView } from '@git-diff-view/react'

import { ExpandIcon, SparklesIcon } from '@gitmono/ui/Icons'
import { cn } from '@gitmono/ui/src/utils'
import { parsedDiffs } from '@/components/DiffView/parsedDiffs'

function calculateDiffStatsFromRawDiff(diffText: string): { additions: number; deletions: number } {
  const lines = diffText.split('\n');

  let additions = 0;

  let deletions = 0;

  for (const line of lines) {
    if (line.startsWith('+') && !line.startsWith('+++')) {
      additions++
    } else if (line.startsWith('-') && !line.startsWith('---')) {
      deletions++
    }
  }

  return { additions, deletions }
}

function generateParsedFiles(diffFiles: { path: string; lang: string; diff: string }[]): { 
  file: { path: string; lang: string; diff: string }; 
  instance: DiffFile; 
  stats: { additions: number; deletions: number } 
}[] {
  return diffFiles.map((file) => {
    const instance = new DiffFile('', '', '', '', [file.diff], file.lang);

    instance.init();
    instance.buildSplitDiffLines();
    instance.buildUnifiedDiffLines();
    const stats = calculateDiffStatsFromRawDiff(file.diff);

    return { file, instance, stats }
  })
}

export default function FileDiff({ diffs }: { diffs: string }) {
  const diffFiles = useMemo(() => parsedDiffs(diffs), [diffs]);

  const parsedFiles = useMemo(() => generateParsedFiles(diffFiles), [diffFiles]);

  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  const [expandedMap, setExpandedMap] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(diffFiles.map((f) => [f.path, false]))
  );
  
  const fileRefs = useRef<Record<string, HTMLDivElement | null>>({});

  const toggleExpanded = (path: string) => {
    setExpandedMap((prev) => ({ ...prev, [path]: !prev[path] }))
  };

  useEffect(() => {
    setExpandedMap(Object.fromEntries(diffFiles.map((f) => [f.path, false])));
  }, [diffFiles]);

  const RenderDiffView = ({ file, instance }: { 
    file: { path: string; lang: string; diff: string };
    instance: DiffFile;
  }) => {
    if (file.lang === 'plaintext') {
      return <div className='text-center p-2'>Binary file</div>
    }

    return (
      <DiffView
        diffFile={instance}
        diffViewFontSize={14}
        diffViewWrap
        diffViewMode={DiffModeEnum.Unified}
        diffViewHighlight
      />
    )
  }

  return (
    <div className='flex font-sans'>
      <div
        className='rounded-lg bg-gray-100 w-[300px] h-[85vh] border border-[#ddd] p-2 overflow-y-auto sticky top-5'
      >
        <ul>
          {parsedFiles.map(({ file }) => (
            <li
              key={file.path}
              onClick={() => {
                setSelectedPath(file.path)
                setExpandedMap((prev) => ({ ...prev, [file.path]: true }))
                const el = fileRefs.current[file.path]

                if (el) {
                  el.scrollIntoView({ behavior: 'smooth', block: 'start' })
                }
              }}
              className={cn('px-2 py-1 text-sm cursor-pointer rounded-md mb-1', selectedPath === file.path ? 'bg-[#e6f0ff]' : 'bg-transparent')}
            >
              {file.path}
            </li>
          ))}
        </ul>
      </div>

      <div className='flex-1 overflow-y-auto px-4'>
        {parsedFiles.map(({ file, instance, stats }) => {
          const isExpanded = expandedMap[file.path]

          return (
            <div
              key={file.path}
              ref={(el) => void (fileRefs.current[file.path] = el)}
              className='mb-4 rounded-lg border border-gray-300'
            >
              <div
                onClick={() => toggleExpanded(file.path)}
                className={cn('flex items-center justify-between px-4 py-2 text-sm', isExpanded && 'border-b border-gray-300')}
              >
                <span className='flex items-center cursor-pointer'>
                  {isExpanded ? (
                    <SparklesIcon className='align-middle text-xl' />
                  ) : (
                    <ExpandIcon className='align-middle text-xl' />
                  )}
                  <span className='ml-1'>{file.path}</span>
                </span>
                <span className='text-xs font-bold'>
                  <span className='text-green-500'>+{stats.additions}</span>{' '}
                  <span className='text-red-500'>−{stats.deletions}</span>
                </span>
              </div>

              {isExpanded && <RenderDiffView file={file} instance={instance} />}
            </div>
          )
        })}
      </div>
    </div>
  )
}
