'use client'

import { useEffect, useRef, useState } from 'react'
import { Highlight, themes, Prism } from 'prism-react-renderer'

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
type ViewMode = 'code' | 'blame'



function UserAvatar({ username, zIndex }: { username?: string ; zIndex?: number }) {
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
}


function UserAvatarGroup({ contributors }: { contributors: Array<{
    email: string
    username?: string | null
  }>
}) {
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
}


const CodeContent = ({ fileContent, path }: { fileContent: string; path?: string[] }) => {
  const [lfs, setLfs] = useState(false)
  const [selectedLine, setSelectedLine] = useState<number | null>(null)
  const [viewMode, setViewMode] = useState<ViewMode>('code')

  const filePath =  path?.join('/') || ''

  useEffect(() => {
    setViewMode('code')
  }, [filePath])


  let filename

  if (!path || path.length === 0) {
    toast.error('Path information is missing')
    filename = ''
  } else {
    filename = path[path.length - 1]
  }
  const detectedLanguage = getLangFromFileName(filename)

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

  const handleLineClick = (lineNumber: number) => {
    setSelectedLine(lineNumber === selectedLine ? null : lineNumber)
  }

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
    // Create a new window/tab with the raw content
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
      <div className="flex items-center justify-between py-2 px-4 bg-gray-50 border-b border-gray-200">

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



  const renderCodeView = () => (
    <Highlight theme={themes.github} code={fileContent} language={detectedLanguage}>
      {({ style, tokens, getLineProps, getTokenProps }) => (
        <pre
          style={{
            ...style,
            backgroundColor: '#fff',
            padding: '16px',
            paddingTop: '30px',
            userSelect: 'text',
            whiteSpace: 'pre',
            wordBreak: 'break-all'
          }}
          className='overflow-x-auto rounded-lg p-4 text-sm'
        >
          {!lfs &&
            tokens.map((line, i) => (
              <div
                /* eslint-disable-next-line react/no-array-index-key */
                key={i}
                {...getLineProps({ line })}
                // @ts-ignore
                ref={(el) => (lineRef.current[i] = el as HTMLDivElement)}
                style={{
                  backgroundColor: selectedLine === i ? '#f0f7ff' : 'transparent'
                }}
                className='flex justify-self-auto'
                onClick={() => handleLineClick(i)}
              >
                  <span className='inline-block w-8'>
                    {selectedLine === i ? (
                        <div></div>
                      ) : // <Dropdown
                      //   menu={{
                      //     ...menuItems,
                      //     onClick: (props) => {
                      //       if (props.key === '1') {
                      //         handleCopyLine(line.map((i) => i.content).join(''))
                      //       }
                      //     }
                      //   }}
                      //   className='rounded border border-gray-200 bg-gray-100'
                      // >
                      //   <Button size={'sm'} className='flex h-6 w-6 p-0' />
                      // </Dropdown>
                      null}
                  </span>
                <span className={styles.codeLineNumber}>{i + 1}</span>
                    {line.map((token, key) => (
                      // eslint-disable-next-line react/no-array-index-key
                      <span key={key} {...getTokenProps({ token })} />
                    ))}
              </div>
            ))}
          {lfs && <span>(Sorry about that, but we can’t show files that are this big right now.)</span>}
          </pre>
      )}
    </Highlight>
  )


  const RenderBlameView = () => {
    const { data: blameData, isLoading: isBlameLoading } = useGetBlame({
      refs: "main",
      path: filePath,
      page:1,
    })

    if (isBlameLoading) {
      return (
        <div className='flex items-center justify-center p-8'>
          <div className='text-gray-500'>Loading blame information...</div>
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


    const getBlameColorClass = (authorTime: number, earliest_commit_time: number, latest_commit_time: number) => {
      if (!authorTime) return styles['bg-blame-1']

      if (earliest_commit_time === latest_commit_time) {
        return styles['bg-blame-10']
      }
      const relativePosition = (authorTime - earliest_commit_time) / (latest_commit_time - earliest_commit_time)
      const colorLevel = Math.min(Math.floor(relativePosition * 10) + 1, 11)

      return styles[`bg-blame-${colorLevel}`]
    }


    const formatRelativeTime = (authorTime: number) => {
      if (!authorTime) return 'Unknown'

      const now = Date.now() / 1000
      const daysDiff = Math.floor((now - authorTime) / (24 * 60 * 60))

      if (daysDiff < 1) return 'Today'
      if (daysDiff < 7) return `${daysDiff} days ago`
      if (daysDiff < 30) return `${Math.floor(daysDiff / 7)} weeks ago`
      if (daysDiff < 365) return `${Math.floor(daysDiff / 30)} months ago`
      return `${Math.floor(daysDiff / 365)} years ago`
    }

    return (
      <>
        <ContributionRecord contributors={ blameData.data?.contributors} />
        <div className="border-t border-gray-200 bg-white overflow-x-auto">
          {blameData.data?.blocks?.map((block, blockIndex) => {
            const colorClass = getBlameColorClass(block.blame_info?.author_time || 0 ,blameData.data?.earliest_commit_time || 0 , blameData.data?.latest_commit_time || 0)
            const blockLines = block.content.split('\n')

            return (
              <div
                key={block?.blame_info.commit_hash}
                className="border-b border-gray-200 hover:bg-blue-50 transition-colors duration-150"
              >
                {blockLines.map((lineContent, lineIndex) => {
                  const absoluteLineNumber = block.start_line + lineIndex
                  const isSelected = selectedLine === (absoluteLineNumber - 1)

                  return (
                    // eslint-disable-next-line react/no-array-index-key
                    <Highlight key={`${blockIndex}-${lineIndex}`} theme={themes.github} code={lineContent} language={detectedLanguage}>
                      {({ tokens, getLineProps, getTokenProps }) => (
                        <div
                          {...getLineProps({ line: tokens[0] })}
                          className="flex min-w-0"
                          onClick={() => handleLineClick(absoluteLineNumber - 1)}
                          style={{
                            backgroundColor: isSelected ? '#f0f7ff' : '#fff',
                            fontSize: '12px',
                            height: '20px',

                          }}
                        >
                          <div className="w-1 flex-shrink-0" >
                            <div className={`${colorClass} rounded-sm h-[90%] w-[95%]` } ></div>
                          </div>


                          {lineIndex === 0 ? (
                            <div className="flex items-center px-3 py-1 bg-white flex-shrink-0" style={{ width: '350px' }}>
                              <span className="w-[100px] text-gray-600 truncate">
                                {formatRelativeTime(block.blame_info?.author_time || 0)}
                              </span>

                              <UserAvatar
                                username={block.blame_info?.author_username || ''}
                                zIndex = {block.blame_info.author_time}
                              />

                              <div className="w-[200px] flex items-center space-x-2 text-xs whitespace-nowrap">
                                <span className="text-gray-600 truncate" title={block.blame_info?.commit_summary}>
                                  {block.blame_info?.commit_message || 'No commit message'}
                                </span>
                              </div>
                            </div>
                          ) : (
                            <div className="bg-white flex-shrink-0" style={{ width: '350px' }}></div>
                          )}

                          <div className="flex items-center justify-center bg-white text-xs text-gray-500 select-none flex-shrink-0 border-r border-gray-200" style={{ width: '60px' }}>
                            {absoluteLineNumber}
                          </div>

                          <div className="flex items-center bg-white font-mono text-sm py-1 pl-3 min-w-0" style={{ minWidth: '0', width: 'max-content' }}>
                            <div className="whitespace-pre">
                              {tokens[0]?.map((token, key) => (
                                // eslint-disable-next-line react/no-array-index-key
                                <span key={key} {...getTokenProps({ token })} />
                              ))}
                            </div>
                          </div>
                        </div>
                      )}
                    </Highlight>
                  )
                })}
              </div>
            )
          })}
        </div>
      </>
    )
  }



  return (
    <div>
      <div className={styles.toolbar}>
        <div className='m-2 h-8 rounded-lg bg-gray-200'>
          <button
            className={`${styles.toolbarLeftButton} ${viewMode === 'code' ? styles.active : ''}`}
            onClick={() => setViewMode('code')}
          >
            Code
          </button>
          <button
            className={`${styles.toolbarLeftButton} ${viewMode === 'blame' ? styles.active : ''}`}
            onClick={() => setViewMode('blame')}
          >
            Blame
          </button>
        </div>
        <span className='m-2 text-gray-500'>{`${fileContent.split('\n').length}   lines  .  2.79  KB`}</span>
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
      {viewMode === 'code' ? renderCodeView() : RenderBlameView()}
    </div>
  )
}

export default CodeContent