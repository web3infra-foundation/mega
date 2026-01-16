'use client'

import { useCallback, useEffect, useMemo, useState } from 'react'
import Markdown from 'react-markdown'
import { codeToTokens } from 'shiki'

import 'github-markdown-css/github-markdown-light.css'

import React from 'react'
import { Avatar } from '@mui/material'
import { motion } from 'framer-motion'
import { useRouter as useNextRouter } from 'next/dist/client/router'
import toast from 'react-hot-toast'

import { UsersIcon } from '@gitmono/ui'

import { useGetBlame } from '@/hooks/useGetBlame'
import { useGetOrganizationMember } from '@/hooks/useGetOrganizationMember'
import { getLanguageForFile } from '@/utils/shikiLanguageFallback'

import BlobEditor from './BlobEditor'
import styles from './CodeContent.module.css'

type ViewMode = 'code' | 'blame' | 'preview'

const UserAvatar = React.memo(({ username, zIndex }: { username?: string; zIndex?: number }) => {
  const { data: memberData } = useGetOrganizationMember({ username })

  return (
    <motion.div>
      <Avatar
        alt={username}
        src={memberData?.user?.avatar_url || ''}
        sx={{ width: 20, height: 20, border: '2px solid #fff' }}
        style={{ zIndex }}
      />
    </motion.div>
  )
})

const UserAvatarGroup = React.memo(
  ({
    contributors
  }: {
    contributors: Array<{
      email: string
      username?: string | null
    }>
  }) => {
    return (
      <motion.div
        className='flex items-center justify-center'
        initial='stacked'
        whileHover='spread'
        style={{
          width: 20,
          height: 20,
          position: 'relative'
        }}
      >
        {contributors.map((c, i) => (
          <motion.div
            key={c.username}
            variants={{
              stacked: { x: -i * 12 },
              spread: { x: i * 3 }
            }}
            transition={{ type: 'spring', stiffness: 300, damping: 20 }}
            style={{ position: 'relative' }}
          >
            <UserAvatar username={c.username || undefined} zIndex={i} />
          </motion.div>
        ))}
      </motion.div>
    )
  }
)

UserAvatar.displayName = 'UserAvatar'
UserAvatarGroup.displayName = 'UserAvatarGroup'

const CodeContent = ({
  fileContent,
  path,
  isCodeLoading
}: {
  fileContent: string
  path?: string[]
  isCodeLoading: boolean
}) => {
  const [lfs, setLfs] = useState(false)
  const [selectedLine, setSelectedLine] = useState<number | null>(null)
  const [manualViewMode, setManualViewMode] = useState<ViewMode | null>(null)
  const [isEditMode, setIsEditMode] = useState(false)

  const [shikiTokens, setShikiTokens] = useState<Array<Array<{ content: string; color?: string }>>>([])
  const [isShikiLoading, setIsShikiLoading] = useState(false)

  const nextRouter = useNextRouter()

  const filePath = useMemo(() => path?.join('/') || '', [path])
  const version = (nextRouter.query.version as string) || 'main'

  const { data: blameData, isLoading: isBlameLoading } = useGetBlame({
    refs: version,
    path: filePath,
    page: 1
  })

  const filename = useMemo(() => {
    if (!path || path.length === 0) {
      return ''
    }
    return path[path.length - 1]
  }, [path])

  const isMarkdownFile = useMemo(() => {
    return filename.toLowerCase().endsWith('.md')
  }, [filename])

  const viewMode = useMemo<ViewMode>(() => {
    if (manualViewMode) return manualViewMode
    return isMarkdownFile ? 'preview' : 'code'
  }, [manualViewMode, isMarkdownFile])

  useEffect(() => {
    setManualViewMode(null)
    setIsEditMode(false)
  }, [path])

  const detectedLanguage = useMemo(() => getLanguageForFile(filename), [filename])

  useEffect(() => {
    if (isCodeLoading) return

    if (!fileContent) {
      setShikiTokens([])
      setIsShikiLoading(false)
      return
    }

    let cancelled = false

    setIsShikiLoading(true)
    codeToTokens(fileContent, {
      lang: detectedLanguage as any,
      theme: 'min-light'
    })
      .then((result) => {
        if (!cancelled) {
          setShikiTokens(result.tokens)
        }
      })
      .catch(() => {
        if (!cancelled) {
          setShikiTokens(fileContent.split('\n').map((line) => [{ content: line, color: '#24292e' }]))
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsShikiLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [fileContent, detectedLanguage, isCodeLoading])

  // const menuItems: MenuProps = {
  //   items: [
  //     {
  //       label: 'Copy line',
  //       key: '1'
  //     },
  //     {
  //       label: 'Copy permalink',
  //       key: '2'
  //     },
  //     {
  //       label: 'View file in GitHub.dev',
  //       key: '3'
  //     },
  //     {
  //       label: 'View file in different branch/tag',
  //       key: '4'
  //     }
  //   ]
  // }

  useEffect(() => {
    setLfs(isLfsContent(fileContent))
  }, [fileContent])

  const handleLineClick = useCallback(
    (lineNumber: number) => {
      setSelectedLine(lineNumber === selectedLine ? null : lineNumber)
    },
    [selectedLine]
  )

  const handleCopyLine = (line: string) => {
    if (navigator.clipboard) {
      navigator.clipboard
        .writeText(line)
        .then(() => toast.success('Copied to clipboard'))
        .catch(() => toast.error('Copied failed'))
    } else {
      const textarea = document.createElement('textarea')

      textarea.value = line
      document.body.appendChild(textarea)
      textarea.select()
      try {
        document.execCommand('copy')
        toast.success('Copied to clipboard')
      } catch {
        toast.error('Copied failed')
      }
    }
  }

  const handleCopy = () => {
    handleCopyLine(fileContent)
  }

  const handleRawView = () => {
    const newWindow = window.open()

    if (newWindow) {
      newWindow.document.write(`
        <!DOCTYPE html>
        <html lang="en">
          <head>
            <title>Raw Content</title>
            <style>
              body {
                font-family: monospace;
                white-space: pre;
                padding: 20px;
              }
            </style>
          </head>
          <body>${fileContent.replace(/</g, '&lt;').replace(/>/g, '&gt;')}</body>
        </html>
      `)
      newWindow.document.close()
    } else {
      toast.error('Unable to open new window. Please check your browser settings.')
    }
  }

  const handleDownload = () => {
    const blob = new Blob([fileContent], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')

    a.href = url
    a.download = filename

    // Append to the document, click it, and remove it
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)

    // Release the URL object
    URL.revokeObjectURL(url)
  }

  function isLfsContent(content: string): boolean {
    const lines = content.split('\n')
    let foundVersion = false
    let foundOid = false
    let foundSize = false

    for (const line of lines) {
      if (line.startsWith('version ')) {
        foundVersion = true
      } else if (line.startsWith('oid sha256:')) {
        foundOid = true
      } else if (line.startsWith('size ')) {
        foundSize = true
      }
      if (foundVersion && foundOid && foundSize) {
        return true
      }
    }
    return false
  }

  const ContributionRecord = ({
    contributors
  }: {
    contributors: Array<{
      email: string
      username?: string | null
    }>
  }) => {
    return (
      <div className='flex items-center justify-between border-b border-gray-200 bg-gray-50 px-4 py-2'>
        <div className='flex items-center space-x-2'>
          <span className='text-xs text-gray-600'>Older</span>
          <div className='flex items-center space-x-1'>
            <div className={`h-3 w-3 ${styles['bg-blame-10']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-9']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-8']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-7']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-6']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-5']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-4']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-3']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-2']}`}></div>
            <div className={`h-3 w-3 ${styles['bg-blame-1']}`}></div>
          </div>
          <span className='text-xs text-gray-600'>Newer</span>
        </div>

        <div className='flex items-center space-x-3'>
          <div className='flex items-center space-x-2'>
            <div className='flex -space-x-1'>
              <div className='flex items-center space-x-1'>
                <UserAvatarGroup contributors={contributors}></UserAvatarGroup>
              </div>

              <div className='flex items-center space-x-1 pl-2 pr-3'>
                <UsersIcon size={16} className='text-black' />
                <span className='ml-0 text-xs text-black'>Contributors</span>
              </div>

              <span className='rounded-full bg-gray-200 px-2 py-1 text-xs text-black'>{contributors?.length || 0}</span>
            </div>
          </div>
        </div>
      </div>
    )
  }

  const getBlameColorClass = useCallback(
    (authorTime: number, earliest_commit_time: number, latest_commit_time: number) => {
      if (!authorTime) return styles['bg-blame-1']

      if (earliest_commit_time === latest_commit_time) {
        return styles['bg-blame-10']
      }
      const relativePosition = (authorTime - earliest_commit_time) / (latest_commit_time - earliest_commit_time)
      const colorLevel = Math.min(Math.floor(relativePosition * 10) + 1, 10)

      return styles[`bg-blame-${colorLevel}`]
    },
    []
  )

  const formatRelativeTime = useCallback((authorTime: number) => {
    if (!authorTime) return 'Unknown'

    const now = Date.now() / 1000
    const daysDiff = Math.floor((now - authorTime) / (24 * 60 * 60))

    if (daysDiff < 1) return 'Today'
    if (daysDiff < 7) return `${daysDiff} days ago`
    if (daysDiff < 30) return `${Math.floor(daysDiff / 7)} weeks ago`
    if (daysDiff < 365) return `${Math.floor(daysDiff / 30)} months ago`
    return `${Math.floor(daysDiff / 365)} years ago`
  }, [])

  const processedBlameBlocks = useMemo(() => {
    if (!blameData?.data?.blocks) return []

    return blameData.data.blocks.map((block) => {
      const colorClass = getBlameColorClass(
        block.blame_info?.commit_time || 0,
        blameData.data?.earliest_commit_time || 0,
        blameData.data?.latest_commit_time || 0
      )
      const blockLines = block.content.split('\n')

      return {
        colorClass,
        blameInfo: block.blame_info,
        startLine: block.start_line,
        lines: blockLines.map((content, index) => ({
          content,
          lineNumber: block.start_line + index
        }))
      }
    })
  }, [blameData, getBlameColorClass])

  // Blame tokens state - map lineNumber to tokens
  const [blameTokensMap, setBlameTokensMap] = useState<Map<number, Array<{ content: string; color?: string }>>>(
    new Map()
  )

  // Load Shiki tokens for blame view - optimized to process per block instead of per line
  useEffect(() => {
    if (!processedBlameBlocks.length) return

    let cancelled = false

    const loadBlameTokens = async () => {
      const results = await Promise.all(
        processedBlameBlocks.map(async (block) => {
          const blockContent = block.lines.map((l) => l.content).join('\n')

          try {
            const result = await codeToTokens(blockContent, {
              lang: detectedLanguage as any,
              theme: 'min-light'
            })

            return { block, tokens: result.tokens }
          } catch {
            return { block, tokens: null }
          }
        })
      )

      if (cancelled) return

      const newMap = new Map<number, Array<{ content: string; color?: string }>>()

      for (const { block, tokens } of results) {
        block.lines.forEach((line, index) => {
          if (tokens) {
            newMap.set(line.lineNumber, tokens[index] || [])
          } else {
            newMap.set(line.lineNumber, [{ content: line.content, color: '#24292e' }])
          }
        })
      }

      setBlameTokensMap(newMap)
    }

    loadBlameTokens()

    return () => {
      cancelled = true
    }
  }, [processedBlameBlocks, detectedLanguage])

  const renderCodeView = useCallback(() => {
    if (isCodeLoading || isShikiLoading) {
      return (
        <div
          className='animate-pulse'
          style={{ flex: 1, backgroundColor: '#fff', padding: '0 16px', overflow: 'hidden' }}
        >
          {Array.from({ length: 10 }).map((_, index) => (
            // eslint-disable-next-line react/no-array-index-key
            <div key={index} className='flex items-center py-1'>
              <div className='mr-4 h-4 w-12 rounded bg-gray-200'></div>
              <div className='h-4 rounded bg-gray-200' style={{ width: `${Math.random() * 50 + 30}%` }}></div>
            </div>
          ))}
        </div>
      )
    }
    if (lfs) {
      return (
        <div className='flex items-center justify-center p-8'>
          {/* eslint-disable-next-line react/no-unescaped-entities */}
          <span>(Sorry about that, but we can't show files that are this big right now.)</span>
        </div>
      )
    }

    return (
      <div className='flex flex-1 flex-col overflow-hidden'>
        <div style={{ flex: 1, overflow: 'auto' }}>
          <div
            className={`rounded-b-lg border border-gray-200 ${styles.codeContainer}`}
            style={{ minWidth: 'fit-content', backgroundColor: '#fff', userSelect: 'text' }}
          >
            {shikiTokens.map((line, index) => {
              return (
                <div
                  // eslint-disable-next-line react/no-array-index-key
                  key={index}
                  style={{
                    backgroundColor: selectedLine === index ? '#f0f7ff' : '#fff',
                    padding: '0 16px',
                    fontSize: '14px',
                    fontFamily: 'monospace',
                    whiteSpace: 'pre',
                    display: 'flex'
                  }}
                  onClick={() => handleLineClick(index)}
                >
                  <span className='inline-block w-8'>{selectedLine === index ? <div></div> : null}</span>
                  <span className={styles.codeLineNumber}>{index + 1}</span>
                  <span style={{ display: 'inline' }}>
                    {line.map((token, key) => (
                      // eslint-disable-next-line react/no-array-index-key
                      <span key={key} style={{ color: token.color, display: 'inline' }}>
                        {token.content}
                      </span>
                    ))}
                  </span>
                </div>
              )
            })}
          </div>
        </div>
      </div>
    )
  }, [lfs, selectedLine, handleLineClick, isCodeLoading, isShikiLoading, shikiTokens])

  const renderBlameView = useCallback(() => {
    if (isBlameLoading) {
      return (
        <div className='animate-pulse' style={{ flex: 1, backgroundColor: '#fff', overflow: 'hidden' }}>
          {Array.from({ length: 10 }).map((_, index) => (
            // eslint-disable-next-line react/no-array-index-key
            <div key={index} className='flex border-b border-gray-200 py-2'>
              <div className='mx-2 w-1 bg-gray-200'></div>

              <div className='flex w-[350px] items-center space-x-2 px-3'>
                <div className='h-4 w-20 rounded bg-gray-200'></div>
                <div className='h-5 w-5 rounded-full bg-gray-300'></div>
                <div className='h-4 w-32 rounded bg-gray-200'></div>
              </div>

              <div className='flex flex-1 items-center px-3'>
                <div className='mr-4 h-4 w-12 rounded bg-gray-200'></div>
                <div className='h-4 rounded bg-gray-200' style={{ width: `${Math.random() * 60 + 20}%` }}></div>
              </div>
            </div>
          ))}
        </div>
      )
    }

    if (!blameData?.data) {
      return (
        <div className='flex items-center justify-center p-8'>
          <div className='text-gray-500'>No blame information available</div>
        </div>
      )
    }

    return (
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <ContributionRecord contributors={blameData.data?.contributors} />
        <div style={{ flex: 1, overflow: 'auto' }}>
          <div
            className={`rounded-b-lg border border-gray-200 ${styles.blameContainer}`}
            style={{ minWidth: 'fit-content', backgroundColor: '#fff', userSelect: 'text' }}
          >
            {processedBlameBlocks.map((block, blockIndex) => {
              const isLastBlock = blockIndex === processedBlameBlocks.length - 1

              return (
                <div
                  // eslint-disable-next-line react/no-array-index-key
                  key={`block-${blockIndex}`}
                  className={`transition-colors duration-150 ${isLastBlock ? '' : 'border-b border-gray-200'}`}
                >
                  <div className='flex'>
                    <div className='flex w-1 flex-shrink-0 items-center'>
                      <div className={`${block.colorClass} h-[99%] w-[95%] rounded-sm`}></div>
                    </div>

                    <div className='flex-shrink-0 border-r border-gray-200' style={{ width: '350px' }}>
                      <div className='top-0 z-10 flex items-center px-3 py-2'>
                        <span className='w-[100px] truncate text-xs text-gray-600'>
                          {formatRelativeTime(block.blameInfo?.commit_time || 0)}
                        </span>
                        <UserAvatar
                          username={block.blameInfo?.author_username || ''}
                          zIndex={block.blameInfo?.commit_time || 0}
                        />
                        <div className='ml-2 flex w-[200px] items-center'>
                          <span className='truncate text-xs text-gray-600' title={block.blameInfo?.commit_summary}>
                            {block.blameInfo?.commit_message || 'No commit message'}
                          </span>
                        </div>
                      </div>
                    </div>

                    <div className={`flex-shrink-0 ${block.lines.length === 1 ? 'flex items-center' : ''}`}>
                      {block.lines.map((line) => {
                        const isSelected = selectedLine === line.lineNumber - 1
                        const lineTokens = blameTokensMap.get(line.lineNumber) || [
                          { content: line.content, color: '#24292e' }
                        ]

                        return (
                          <div
                            key={`line-${line.lineNumber}`}
                            className='flex'
                            onClick={() => handleLineClick(line.lineNumber - 1)}
                            style={{
                              backgroundColor: isSelected ? '#f0f7ff' : '#fff',
                              fontSize: '12px',
                              height: '20px'
                            }}
                          >
                            <div
                              className='flex flex-shrink-0 select-none items-center justify-center bg-white text-xs text-gray-500'
                              style={{ width: '60px' }}
                            >
                              {line.lineNumber}
                            </div>

                            <div className='py-1 pl-3 pr-4 font-mono text-sm' style={{ whiteSpace: 'pre' }}>
                              <span style={{ display: 'inline' }}>
                                {lineTokens.map((token, key) => (
                                  // eslint-disable-next-line react/no-array-index-key
                                  <span key={key} style={{ color: token.color, display: 'inline' }}>
                                    {token.content}
                                  </span>
                                ))}
                              </span>
                            </div>
                          </div>
                        )
                      })}
                    </div>
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      </div>
    )
  }, [
    isBlameLoading,
    blameData,
    processedBlameBlocks,
    selectedLine,
    blameTokensMap,
    handleLineClick,
    formatRelativeTime
  ])

  const renderPreviewView = useCallback(() => {
    if (isCodeLoading) {
      return (
        <div className='animate-pulse p-8' style={{ flex: 1, backgroundColor: '#fff', overflow: 'hidden' }}>
          {Array.from({ length: 3 }).map((_, index) => (
            // eslint-disable-next-line react/no-array-index-key
            <div key={index} className='mb-4'>
              <div className='mb-2 h-6 rounded bg-gray-200' style={{ width: `${Math.random() * 30 + 40}%` }}></div>
              <div className='mb-1 h-4 rounded bg-gray-100' style={{ width: `${Math.random() * 20 + 70}%` }}></div>
              <div className='mb-1 h-4 rounded bg-gray-100' style={{ width: `${Math.random() * 20 + 80}%` }}></div>
              <div className='h-4 rounded bg-gray-100' style={{ width: `${Math.random() * 30 + 50}%` }}></div>
            </div>
          ))}
        </div>
      )
    }

    return (
      <div
        className={`markdown-body overflow-auto rounded-b-lg border border-gray-200 p-8 ${styles.previewContainer}`}
        style={{ userSelect: 'text' }}
      >
        <Markdown>{fileContent}</Markdown>
      </div>
    )
  }, [fileContent, isCodeLoading])

  const handleEditClick = useCallback(() => {
    setIsEditMode(true)
  }, [])

  const handleCancelEdit = useCallback(() => {
    setIsEditMode(false)
  }, [])

  if (isEditMode) {
    return <BlobEditor fileContent={fileContent} filePath={filePath} fileName={filename} onCancel={handleCancelEdit} />
  }

  return (
    <div className='flex flex-1 flex-col overflow-hidden'>
      <div className={`${styles.toolbar} rounded-t-lg border-t border-gray-200`} style={{ flexShrink: 0 }}>
        <div className='m-2 h-8 rounded-lg bg-gray-200'>
          {isMarkdownFile && (
            <button
              className={`${styles.toolbarLeftButton} ${viewMode === 'preview' ? styles.active : ''}`}
              onClick={() => setManualViewMode('preview')}
            >
              Preview
            </button>
          )}
          <button
            className={`${styles.toolbarLeftButton} ${viewMode === 'code' ? styles.active : ''}`}
            onClick={() => setManualViewMode('code')}
          >
            Code
          </button>
          <button
            className={`${styles.toolbarLeftButton} ${viewMode === 'blame' ? styles.active : ''}`}
            onClick={() => setManualViewMode('blame')}
          >
            Blame
          </button>
        </div>
        <div className='hidden whitespace-nowrap text-gray-500 2xl:inline'>
          <span
            className='m-2'
            style={{ minWidth: 0 }}
          >{`${fileContent.split('\n').length}   lines  .  2.79  KB`}</span>
        </div>

        <div className='flex-1' />
        <div className='m-2 h-8 rounded-lg border border-gray-200 p-1'>
          <button className={styles.toolbarRightButton} onClick={handleRawView}>
            Raw
          </button>
          <button className={styles.toolbarRightButton} onClick={handleCopy}>
            Copy
          </button>
          <button className={styles.toolbarRightButton} onClick={handleDownload}>
            Download
          </button>
        </div>
        {version === 'main' && !lfs && (
          <div className='m-2 h-8 rounded-lg border border-gray-200 p-1'>
            <button className={styles.toolbarRightButton} onClick={handleEditClick}>
              Edit
            </button>
          </div>
        )}
      </div>
      {viewMode === 'preview' && renderPreviewView()}
      {viewMode === 'code' && renderCodeView()}
      {viewMode === 'blame' && renderBlameView()}
    </div>
  )
}

export default CodeContent
