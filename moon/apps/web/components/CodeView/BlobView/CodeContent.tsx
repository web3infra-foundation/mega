'use client'
import 'github-markdown-css/github-markdown-light.css'

import { useEffect, useRef, useState } from 'react'
import { ListIcon } from '@gitmono/ui'
import { Button, Dropdown, MenuProps, message} from 'antd'
import { Highlight, themes } from 'prism-react-renderer'

import styles from './CodeContent.module.css'

// @ts-ignore
const CodeContent = ({ fileContent, path }: { fileContent: string, path?: string[] }) => {
  const [lfs, setLfs] = useState(false)
  const [selectedLine, setSelectedLine] = useState<number | null>(null)

  const menuItems: MenuProps = {
    items: [
      {
        label: 'Copy line',
        key: '1'
      }
    ]
  }

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
      navigator.clipboard.writeText(line)
        .then(() => message.success('Copied to clipboard'))
        .catch(() => message.error('Copied failed'))
    } else {
      const textarea = document.createElement('textarea');

      textarea.value = line;
      document.body.appendChild(textarea);
      textarea.select();
      try {
        document.execCommand('copy');
        message.success('Copied to clipboard');
      } catch {
        message.error('Copied failed');
      }
    }
  }

  const handleCopy = () => {
    handleCopyLine(fileContent)
  }

  let filename;

  if (!path || path.length === 0) {
    message.error('Path information is missing');
    filename = "";
  }
  else {
    filename = path[path.length - 1]
  }
  const handleRawView = () => {
    // Create a new window/tab with the raw content
    const newWindow = window.open();

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
      `);
      newWindow.document.close();
    } else {
      message.error('Unable to open new window. Please check your browser settings.');
    }
  }
  const handleDownload = () => {
    const blob = new Blob([fileContent], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');

    a.href = url;
    a.download = filename;

    // Append to the document, click it, and remove it
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);

    // Release the URL object
    URL.revokeObjectURL(url);
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
          <button className={styles.toolbarRightButton} onClick={handleRawView}>Raw</button>
          <button className={styles.toolbarRightButton} onClick={handleCopy}>Copy</button>
          <button className={styles.toolbarRightButton} onClick={handleDownload}>Download</button>
        </div>
        <div className='m-2 h-8 rounded-lg border border-gray-200 p-1'>
          <button className={styles.toolbarRightButton}>Edit</button>
        </div>
      </div>
      {/*todo: Dynamic support for language types*/}
      <Highlight theme={themes.github} code={fileContent} language='rust'>
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
                >
                  <div className='h-6 justify-center items-center align-middle'>
                    <span className='inline-block w-8 text-center justify-center items-center'>
                      {selectedLine === i ?
                        <Dropdown
                          menu={{
                            ...menuItems,
                            onClick: (props) => {
                              if(props.key === '1') {
                                handleCopyLine(line.map(i=>i.content).join(''))
                              }
                            }
                          }}
                          className='bg-gray-100 border rounded border-gray-200 justify-center'

                        >
                          <Button icon={<ListIcon />} size={'small'}/>
                        </Dropdown>
                        :
                        null
                      }
                    </span>
                    <span className={styles.codeLineNumber} onClick={() => handleLineClick(i)}>
                      {i + 1}
                    </span>
                    {line.map((token, key) => (
                      // eslint-disable-next-line react/no-array-index-key
                      <span key={key} {...getTokenProps({ token })} />
                    ))}
                  </div>
                </div>
              ))}
            {lfs && <span>(Sorry about that, but we can’t show files that are this big right now.)</span>}
          </pre>
        )}
      </Highlight>
    </div>
  )
}

export default CodeContent;
