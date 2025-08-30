import { useEffect, useMemo, useRef, useState } from 'react'
import { DiffFile, DiffModeEnum, DiffView } from '@git-diff-view/react'

import { CommonResultFilesChangedPage, DiffItem } from '@gitmono/types/generated'
import { ExpandIcon, SparklesIcon } from '@gitmono/ui/Icons'
import { cn } from '@gitmono/ui/src/utils'

import { parsedDiffs } from '@/components/DiffView/parsedDiffs'

import StableTreeView from './StableTreeView'

function calculateDiffStatsFromRawDiff(diffText: string): { additions: number; deletions: number } {
  const lines = diffText.split('\n')

  let additions = 0

  let deletions = 0

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
  file: { path: string; lang: string; diff: string }
  instance: DiffFile | null
  stats: { additions: number; deletions: number }
}[] {
  return diffFiles.map((file) => {
    if (file.lang === 'binary' || file.diff === 'EMPTY_DIFF_MARKER\n') {
      return {
        file,
        instance: null,
        stats: { additions: 0, deletions: 0 }
      }
    }

    const instance = new DiffFile('', '', '', '', [file.diff], file.lang)

    try {
      instance.init()
      instance.buildSplitDiffLines()
      instance.buildUnifiedDiffLines()
    } catch (e) {
      /* eslint-disable-next-line no-console */
      console.error('error:', e)
    }

    const stats = calculateDiffStatsFromRawDiff(file.diff)

    return { file, instance, stats }
  })
}

export default function FileDiff({
  diffs,
  treeData
}: {
  diffs: DiffItem[]
  treeData: CommonResultFilesChangedPage['data']
}) {
  const diffFiles = useMemo(() => parsedDiffs(diffs), [diffs])

  const parsedFiles = useMemo(() => generateParsedFiles(diffFiles), [diffFiles])

  const [_selectedPath, setSelectedPath] = useState<string | null>(null)

  const [expandedMap, setExpandedMap] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(diffFiles.map((f) => [f.path, false]))
  )

  const fileRefs = useRef<Record<string, HTMLDivElement | null>>({})

  const toggleExpanded = (path: string) => {
    setExpandedMap((prev) => ({ ...prev, [path]: !prev[path] }))
  }

  useEffect(() => {
    setExpandedMap(Object.fromEntries(diffFiles.map((f) => [f.path, false])))
  }, [diffFiles])

  const RenderDiffView = ({
    file,
    instance
  }: {
    file: { path: string; lang: string; diff: string }
    instance: DiffFile | null
  }) => {
    if (instance){
      return (
        <DiffView
          diffFile={instance}
          diffViewFontSize={14}
          diffViewWrap
          diffViewMode={DiffModeEnum.Unified}
          diffViewHighlight
        />
      )
    }else if (file.lang === 'binary') {
      return <div className='p-2 text-center'>Binary file</div>
    } else if (file.diff === 'EMPTY_DIFF_MARKER\n') {
      return <div className='p-2 text-center'>No change</div>
    }

    return null
  }

  return (
    <div className='mt-3 flex font-sans'>
      <div className='sticky top-5 h-[85vh] w-[300px] overflow-y-auto rounded-lg p-2'>
        {treeData?.mui_trees && (
          <StableTreeView
            treeData={treeData?.mui_trees}
            handleClick={(file) => {
              setSelectedPath(file)
              setExpandedMap((prev) => ({ ...prev, [file]: true }))
              const el = fileRefs.current[file]

              if (el) {
                el.scrollIntoView({ behavior: 'smooth', block: 'start' })
              }
            }}
          />
        )}
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
                className={cn(
                  'flex items-center justify-between px-4 py-2 text-sm',
                  isExpanded && 'border-b border-gray-300'
                )}
              >
                <span className='flex cursor-pointer items-center'>
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

              <div className='copyable-text'>{isExpanded && <RenderDiffView file={file} instance={instance} />}</div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
