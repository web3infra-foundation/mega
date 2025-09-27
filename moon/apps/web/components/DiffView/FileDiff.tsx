import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { DiffFile, DiffModeEnum, DiffView } from '@git-diff-view/react'

import { ExpandIcon, SparklesIcon } from '@gitmono/ui/Icons'
import { cn } from '@gitmono/ui/src/utils'

import { parsedDiffs } from '@/components/DiffView/parsedDiffs'

import { DiffItem } from './parsedDiffs'
import { Virtuoso } from "react-virtuoso";
import { LoadingSpinner } from "@gitmono/ui";
import { useGetMrFileChanged } from "@/hooks/useGetMrFileChanged";
import FileTree from "@/components/DiffView/TreeView/FileTree";

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

export default function FileDiff({ id }: {
  id: string
}) {
  const virtuosoRef = useRef(null)

  const [page, setPage] = useState(1)

  const [pageDataMap, setPageDataMap] = useState<Map<number, DiffItem[]>>(new Map())
  const [totalCount, setTotalCount] = useState(0)

  const { fileChanged: MrFilesChangedData, isLoading: isFileChangeLoading } = useGetMrFileChanged(id, {
    page,
    per_page: 100
  })

  useEffect(() => {
    if (MrFilesChangedData?.items && !isFileChangeLoading) {
      setPageDataMap(prev => {
        const newMap = new Map(prev)

        if (!newMap.has(page)) {
          newMap.set(page, MrFilesChangedData.items)
        }
        return newMap
      })

      setTotalCount(MrFilesChangedData.total)
    }
  }, [MrFilesChangedData, isFileChangeLoading, page])

  const fileDiff = useMemo(() => {
    const allItems: DiffItem[] = []

    for (let i = 1; i <= page; i++) {
      const pageData = pageDataMap.get(i)

      if (pageData) {
        allItems.push(...pageData)
      }
    }
    return allItems
  }, [pageDataMap, page])

  const diffFiles = useMemo(() => parsedDiffs(fileDiff), [fileDiff])

  const parsedFiles = useMemo(() => generateParsedFiles(diffFiles), [diffFiles])

  const [expandedMap, setExpandedMap] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(diffFiles.map((f) => [f.path, false]))
  )

  const fileRefs = useRef<Record<string, HTMLDivElement | null>>({})

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const toggleExpanded = (path: string) => {
    setExpandedMap((prev) => ({ ...prev, [path]: !prev[path] }))
  }

  const scrollToFile = useCallback((filePath: string) => {
    // Find if the file exists in current rendered files
    const fileExists = parsedFiles.some(file => file.file.path === filePath)

    if (fileExists) {
      setExpandedMap((prev) => ({ ...prev, [filePath]: true }))

      // Use setTimeout to ensure DOM is updated after expansion
      setTimeout(() => {
        const fileElement = fileRefs.current[filePath]

        if (fileElement) {
          fileElement.scrollIntoView({ behavior: 'smooth', block: 'start' })
        }
      }, 100)
    }
  }, [parsedFiles])

  const hasMoreData = useMemo(() => {
    return fileDiff.length < totalCount
  }, [fileDiff.length, totalCount])

  const loadMoreDiffs = useCallback(() => {
    if (isFileChangeLoading || !hasMoreData) return
    setPage((prev) => prev + 1)
  }, [isFileChangeLoading, hasMoreData])

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
    if (instance) {
      return (
        <DiffView
          diffFile={ instance }
          diffViewFontSize={ 14 }
          diffViewWrap
          diffViewMode={ DiffModeEnum.Unified }
          diffViewHighlight
        />
      )
    } else if (file.lang === 'binary') {
      return <div className='p-2 text-center'>Binary file</div>
    } else if (file.diff === 'EMPTY_DIFF_MARKER\n') {
      return <div className='p-2 text-center'>No change</div>
    }

    return null
  }

  const DiffItem = (index: number) => {
    const { file, instance, stats } = parsedFiles[index]
    const isExpanded = expandedMap[file.path]

    return (
      <div
        id={ file.path }
        key={ file.path }
        ref={ (el) => void (fileRefs.current[file.path] = el) }
        className='mb-4 rounded-lg border border-gray-300 w-full'
      >
        <div
          onClick={ () => toggleExpanded(file.path) }
          className={ cn(
            'flex items-center justify-between px-4 py-2 text-sm',
            isExpanded && 'border-b border-gray-300'
          ) }
        >
          <span className='flex cursor-pointer items-center'>
            { isExpanded? (
              <SparklesIcon className='align-middle text-xl'/>
            ) : (
              <ExpandIcon className='align-middle text-xl'/>
            ) }
            <span className='ml-1'>{ file.path }</span>
          </span>
          <span className='text-xs font-bold'>
            <span className='text-green-500'>+{ stats.additions }</span>
            { ' ' }
            <span className='text-red-500'>−{ stats.deletions }</span>
          </span>
        </div>

        <div className='copyable-text'>
          { isExpanded && <RenderDiffView file={ file } instance={ instance }/> }
        </div>
      </div>
    )
  }

  return (
    <div className='mt-3 flex font-sans'>
      <div className='sticky top-5 h-[80vh] w-[300px] overflow-y-auto rounded-lg p-2'>
        <FileTree link={id} onFileClick={scrollToFile}/>
      </div>

      <div className='flex-1 px-4 w-full h-full'>
        { (totalCount === 0 && !isFileChangeLoading)? (
          <div className='flex h-[85vh] items-center justify-center'>
            No File Changed
          </div>
        ) : (
          <Virtuoso
            ref={ virtuosoRef }
            style={{ height: '76vh', paddingBottom: '20px' }}
            totalCount={ parsedFiles.length }
            itemContent={ DiffItem }
            endReached={ loadMoreDiffs }
            components={ { Footer: () => (isFileChangeLoading && <LoadingSpinner />) } }
            increaseViewportBy={ 350 }
          />
        ) }
      </div>
    </div>
  )
}
