'use client'

import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import { Highlight, themes, Prism } from 'prism-react-renderer'
import { Virtuoso } from 'react-virtuoso'
import Markdown from 'react-markdown'

(typeof global !== "undefined" ? global : window).Prism = Prism

import 'github-markdown-css/github-markdown-light.css'
import { motion } from 'framer-motion';
import { Avatar } from '@mui/material';

import toast from 'react-hot-toast'

import styles from './CodeContent.module.css'

import { useGetBlame } from '@/hooks/useGetBlame'
import { useGetOrganizationMember } from '@/hooks/useGetOrganizationMember'
import { getLangFromFileName } from '@/utils/getLanguageDetection'
import { UsersIcon } from '@gitmono/ui'
import { usePrismLanguageLoader } from '@/hooks/usePrismLanguageLoader'
import React from 'react'
type ViewMode = 'code' | 'blame' | 'preview'



const UserAvatar = React.memo(({ username, zIndex }: { username?: string; zIndex?: number }) => {
  const { data: memberData } = useGetOrganizationMember({ username });

  return (
    <motion.div
    >
      <Avatar
        alt={username}
        src={memberData?.user?.avatar_url || ""}
        sx={{ width: 20, height: 20, border: '2px solid #fff' }}
        style={{ zIndex }}
      />
    </motion.div>
  );
})



const UserAvatarGroup = React.memo(({ contributors }: {
  contributors: Array<{
    email: string
    username?: string | null
  }>
})=> {
  return (
    <motion.div
      className="flex justify-center items-center"
      initial="stacked"
      whileHover="spread"
      style={{
        width: 20,
        height: 20,
        position: "relative",
      }}
    >
      {contributors.map((c, i) => (
        <motion.div
          key={c.username}
          variants={{
            stacked: { x: -i * 12 },
            spread: { x: i * 3 },
          }}
          transition={{ type: "spring", stiffness: 300, damping: 20 }}
          style={{ position: "relative" }}
        >
          <UserAvatar username={c.username || undefined} zIndex={i} />
        </motion.div>
      ))}
    </motion.div>
  );
})

UserAvatar.displayName = 'UserAvatar';
UserAvatarGroup.displayName = 'UserAvatarGroup';


const CodeContent = ({ fileContent, path, isCodeLoading }: 
  { fileContent: string; path?: string[]; isCodeLoading: boolean;
}) => {
  const [lfs, setLfs] = useState(false)
  const [selectedLine, setSelectedLine] = useState<number | null>(null)
  const [manualViewMode, setManualViewMode] = useState<ViewMode | null>(null)

  const filePath = useMemo(() => path?.join('/') || '', [path]);

  const { data: blameData, isLoading: isBlameLoading } = useGetBlame({
    refs: "main",
    path: filePath,
    page:1,
  })

  const filename = useMemo(() => {
    if (!path || path.length === 0) {
      return '';
    }
    return path[path.length - 1];
  }, [path]);

  const isMarkdownFile = useMemo(() => {
    return filename.toLowerCase().endsWith('.md');
  }, [filename]);

  const viewMode = useMemo<ViewMode>(() => {
    if (manualViewMode) 
      return manualViewMode;
    return isMarkdownFile ? 'preview' : 'code';
  }, [manualViewMode, isMarkdownFile]);

  useEffect(() => {
    setManualViewMode(null);
  }, [path])

  const detectedLanguage = useMemo(() => getLangFromFileName(filename), [filename]);

  usePrismLanguageLoader(detectedLanguage)


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
    if (isLfsContent(fileContent)) {
      setLfs(true)
    }
  }, [fileContent])

  const lineRef = useRef<HTMLDivElement[]>([])

  const handleLineClick = useCallback((lineNumber: number) => {
    setSelectedLine(lineNumber === selectedLine ? null : lineNumber)
  }, [
    selectedLine
  ])

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


  const ContributionRecord = ({ contributors }: {
    contributors: Array<{
      email: string
      username?: string | null
    }>
  }) => {
    return (
      <div className="flex items-center justify-between py-2 px-4  bg-gray-50  border-b border-gray-200">

        <div className="flex items-center space-x-2">
          <span className="text-xs text-gray-600">Older</span>
          <div className="flex items-center space-x-1">
            <div className={`w-3 h-3 ${styles['bg-blame-10']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-9']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-8']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-7']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-6']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-5']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-4']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-3']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-2']}`}></div>
            <div className={`w-3 h-3 ${styles['bg-blame-1']}`}></div>
          </div>
          <span className="text-xs text-gray-600">Newer</span>
        </div>

        <div className="flex items-center space-x-3">
          <div className="flex items-center space-x-2">
            <div className="flex -space-x-1">

              <div className="flex items-center space-x-1 ">
                < UserAvatarGroup contributors={ contributors}></UserAvatarGroup>
              </div>


              <div className="flex items-center space-x-1 pr-3 pl-2">
                <UsersIcon size={16} className="text-black" />
                <span className="text-xs text-black ml-0 ">Contributors</span>
              </div>

              <span className="text-xs text-black bg-gray-200 rounded-full px-2 py-1 ">
                  {(contributors?.length || 0)}
              </span>

            </div>
          </div>
        </div>
      </div>
    )
  }

  const getBlameColorClass = useCallback((authorTime: number, earliest_commit_time: number, latest_commit_time: number) => {
    if (!authorTime) return styles['bg-blame-1']

    if (earliest_commit_time === latest_commit_time) {
      return styles['bg-blame-10']
    }
    const relativePosition = (authorTime - earliest_commit_time) / (latest_commit_time - earliest_commit_time)
    const colorLevel = Math.min(Math.floor(relativePosition * 10) + 1, 10)

    return styles[`bg-blame-${colorLevel}`]
  }, [])


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
        block.blame_info?.author_time || 0,
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
  }, [
    blameData,
    getBlameColorClass
  ])



  const renderCodeView = useCallback(() => {

    if (isCodeLoading) {
      return (
        <div className="animate-pulse" style={{ height: 'calc(100vh - 215px)', backgroundColor: '#fff', padding: '0 16px' }}>
        {Array.from({ length: 10 }).map((_, index) => (
          // eslint-disable-next-line react/no-array-index-key
          <div key={index} className="flex items-center py-1">
            <div className="w-12 h-4 bg-gray-200 rounded mr-4"></div>
            <div
              className="h-4 bg-gray-200 rounded" 
              style={{ width: `${Math.random() * 50 + 30}%` }}
            ></div>
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
      <div style={{ height: 'calc(100vh - 215px)', overflowX: 'auto', overflowY: 'hidden' }}>
        <Highlight theme={themes.github} code={fileContent} language={detectedLanguage}>
          {({ style, tokens, getLineProps, getTokenProps }) => (
            <Virtuoso
              style={{
                height: 'calc(100vh - 215px)',
                backgroundColor: '#fff',
              }}
              totalCount={tokens.length}
              itemContent={(index) => {
                const line = tokens[index]

                return (
                  <div
                    key={index}
                    {...getLineProps({ line })}
                    ref={(el) => {
                      if (el) lineRef.current[index] = el
                    }}
                    style={{
                      ...style,
                      backgroundColor: selectedLine === index ? '#f0f7ff' : '#fff',
                      padding: '0 16px',
                      fontSize: '14px',
                      fontFamily: 'monospace',
                      whiteSpace: 'pre',
                      display: 'flex',
                      minWidth: 'fit-content',
                    }}
                    onClick={() => handleLineClick(index)}
                  >
                    <span className='inline-block w-8'>
                      {selectedLine === index ? (
                        <div></div>
                      ) : null}
                    </span>
                    <span className={styles.codeLineNumber}>{index + 1}</span>
                    <span style={{ display: 'inline' }}>
                      {line.map((token, key) => (
                        // eslint-disable-next-line react/no-array-index-key
                        <span key={key} {...getTokenProps({ token })} style={{ display: 'inline' }} />
                      ))}
                    </span>
                  </div>
                )
              }}
            />
          )}
        </Highlight>
      </div>
    )
  }, [
    fileContent,
    detectedLanguage,
    lfs,
    selectedLine,
    handleLineClick,
    isCodeLoading
  ])


  const renderBlameView = useCallback(() => {
    if (isBlameLoading) {
      return (
        <div className="animate-pulse" style={{ height: 'calc(100vh - 255px)', backgroundColor: '#fff' }}>
          {Array.from({ length: 10 }).map((_, index) => (
            // eslint-disable-next-line react/no-array-index-key
            <div key={index} className="flex border-b border-gray-200 py-2">

              <div className="w-1 bg-gray-200 mx-2"></div>

              <div className="w-[350px] flex items-center px-3 space-x-2">
                <div className="w-20 h-4 bg-gray-200 rounded"></div>
                <div className="w-5 h-5 bg-gray-300 rounded-full"></div>
                <div className="w-32 h-4 bg-gray-200 rounded"></div>
              </div>

              <div className="flex-1 flex items-center px-3">
                <div className="w-12 h-4 bg-gray-200 rounded mr-4"></div>
                <div 
                  className="h-4 bg-gray-200 rounded" 
                  style={{ width: `${Math.random() * 60 + 20}%` }}
                ></div>
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
      <>
        <ContributionRecord contributors={blameData.data?.contributors} />
        <Virtuoso
          style={{
            height: 'calc(100vh - 255px)',
            backgroundColor: '#fff',
          }}
          totalCount={processedBlameBlocks.length}
          itemContent={(blockIndex) => {
            const block = processedBlameBlocks[blockIndex]

            return (
              <div
                key={`block-${blockIndex}`}
                className="border-b border-gray-200  transition-colors duration-150"
              >
                <div className="flex min-w-0">

                  <div className="flex-shrink-0 w-1 flex items-center" >
                    <div className={`${block.colorClass} rounded-sm h-[99%] w-[95%]` } ></div>
                  </div>


                  <div className="flex-shrink-0 border-r border-gray-200" style={{ width: '350px' }}>
                    <div className="flex items-center px-3 py-2   top-0 z-10 ">
                      <span className="w-[100px] text-xs text-gray-600 truncate">
                        {formatRelativeTime(block.blameInfo?.author_time || 0)}
                      </span>
                      <UserAvatar
                        username={block.blameInfo?.author_username || ''}
                        zIndex={block.blameInfo?.author_time || 0}
                      />
                      <div className="w-[200px] flex items-center ml-2">
                        <span
                          className="text-xs text-gray-600 truncate"
                          title={block.blameInfo?.commit_summary}
                        >
                          {block.blameInfo?.commit_message || 'No commit message'}
                        </span>
                      </div>
                    </div>
                  </div>


                  <div className={`flex-1 min-w-0 ${block.lines.length === 1 ? 'flex items-center' : ''}`}>
                    {block.lines.map((line) => {
                      const isSelected = selectedLine === (line.lineNumber - 1)

                      return (
                        <Highlight
                          key={`line-${line.lineNumber}`}
                          theme={themes.github}
                          code={line.content}
                          language={detectedLanguage}
                        >
                          {({ tokens, getLineProps, getTokenProps }) => (
                            <div
                              {...getLineProps({ line: tokens[0] })}
                              className="flex min-w-0"
                              onClick={() => handleLineClick(line.lineNumber - 1)}
                              style={{
                                backgroundColor: isSelected ? '#f0f7ff' : '#fff',
                                fontSize: '12px',
                                height: '20px',
                              }}
                            >

                              <div
                                className="flex items-center justify-center text-xs text-gray-500 select-none flex-shrink-0 bg-white"
                                style={{ width: '60px' }}
                              >
                                {line.lineNumber}
                              </div>

                              <div
                                className="flex items-center font-mono text-sm py-1 pl-3 min-w-0"
                                style={{
                                  minWidth: '0',
                                  width: 'max-content'
                                }}
                              >
                                <div className="whitespace-pre" style={{ display: 'inline' }}>
                                  {tokens[0]?.map((token, key) => (
                                    // eslint-disable-next-line react/no-array-index-key
                                    <span key={key} {...getTokenProps({ token })} style={{ display: 'inline' }} />
                                  ))}
                                </div>
                              </div>
                            </div>
                          )}
                        </Highlight>
                      )
                    })}
                  </div>
                </div>
              </div>
            )
          }}
        />
      </>
    )
  }, [
    isBlameLoading,
    blameData,
    processedBlameBlocks,
    selectedLine,
    detectedLanguage,
    handleLineClick,
    formatRelativeTime
  ])

  const renderPreviewView = useCallback(() => {
    if (isCodeLoading) {
      return (
        <div className="animate-pulse p-8" style={{ height: 'calc(100vh - 215px)', backgroundColor: '#fff' }}>
          {Array.from({ length: 3 }).map((_, index) => (
            // eslint-disable-next-line react/no-array-index-key
            <div key={index} className="mb-4">
              <div className="h-6 bg-gray-200 rounded mb-2" style={{ width: `${Math.random() * 30 + 40}%` }}></div>
              <div className="h-4 bg-gray-100 rounded mb-1" style={{ width: `${Math.random() * 20 + 70}%` }}></div>
              <div className="h-4 bg-gray-100 rounded mb-1" style={{ width: `${Math.random() * 20 + 80}%` }}></div>
              <div className="h-4 bg-gray-100 rounded" style={{ width: `${Math.random() * 30 + 50}%` }}></div>
            </div>
          ))}
        </div>
      )
    }

    return (
      <div 
        className='markdown-body p-8' 
        style={{ 
          height: 'calc(100vh - 215px)', 
          overflow: 'auto',
          backgroundColor: '#fff'
        }}
      >
        <Markdown>{fileContent}</Markdown>
      </div>
    )
  }, [fileContent, isCodeLoading])

  return (
    <div>
      <div className={styles.toolbar}>
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
        <div className='text-gray-500 whitespace-nowrap hidden 2xl:inline'>
          <span className='m-2' style={{ minWidth: 0 }}>{`${fileContent.split('\n').length}   lines  .  2.79  KB`}</span>
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
        <div className='m-2 h-8 rounded-lg border border-gray-200 p-1'>
          <button className={styles.toolbarRightButton}>Edit</button>
        </div>
      </div>
      {viewMode === 'preview' && renderPreviewView()}
      {viewMode === 'code' && renderCodeView()}
      {viewMode === 'blame' && renderBlameView()}

    </div>
  )
}

export default CodeContent