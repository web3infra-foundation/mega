'use client'

import { useEffect, useRef, useState } from 'react'
import { Highlight, themes } from 'prism-react-renderer'

import 'github-markdown-css/github-markdown-light.css'

import toast from 'react-hot-toast'

import styles from './CodeContent.module.css'

const suffixToLangMap: Record<string, string> = {
  '.js': 'jsx',
  '.jsx': 'jsx',
  '.tsx': 'tsx',
  '.kt': 'kotlin',
  '.json': 'json',
  '.md': 'markdown',
  '.py': 'python',
  '.rs': 'rust',
  '.cpp': 'cpp',
  '.h': 'cpp',
  '.go': 'go',
  '.yml': 'yaml',
  '.yaml': 'yaml'
}

function getLangFromFileName(fileName: string): string {
  const lastPart = fileName.toLowerCase().match(/\.[^./\\]+$/)

  if (lastPart) {
    return suffixToLangMap[lastPart[0].toLowerCase()] ?? 'markdown'
  }
  return 'markdown'
}

const CodeContent = ({ fileContent, path }: { fileContent: string; path?: string[] }) => {
  const [lfs, setLfs] = useState(false)
  const [selectedLine, setSelectedLine] = useState<number | null>(null)

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

  let filename

  if (!path || path.length === 0) {
    toast.error('Path information is missing')
    filename = ''
  } else {
    filename = path[path.length - 1]
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

  return (
    <div>
      <div className={styles.toolbar}>
        <div className='m-2 h-8 rounded-lg bg-gray-200'>
          <button className={`${styles.toolbarLeftButton} ${styles.active}`} defaultChecked={true}>
            Code
          </button>
          <button className={styles.toolbarLeftButton}>Blame</button>
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
      <Highlight theme={themes.github} code={fileContent} language={getLangFromFileName(filename)}>
        {({ style, tokens, getLineProps, getTokenProps }) => (
          <pre
            style={{
              ...style,
              backgroundColor: '#fff',
              padding: '16px',
              paddingTop: '30px',
              userSelect: 'text'
            }}
            className='overflow-x-auto whitespace-pre rounded-lg p-4 text-sm'
          >
            {/* <Button icon={<DotsHorizontal />} size={'sm'} className='flex h-6 w-6 p-0' /> */}
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
                  className='flex h-6 items-center'
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
    </div>
  )
}

export default CodeContent
