import Editor from '@/components/CodeView/BlobView/Editor'

import 'github-markdown-css/github-markdown-light.css'

import { useEffect, useRef, useState } from 'react'
import { Highlight, themes } from 'prism-react-renderer'
import { createRoot } from 'react-dom/client'

import styles from './CodeContent.module.css'

// @ts-ignore
const CodeContent = ({ fileContent }: string) => {
  const [showEditor, setShowEditor] = useState(false)
  const [lfs, setLfs] = useState(false)

  useEffect(() => {
    if (isLfsContent(fileContent)) {
      setLfs(true)
    }
  }, [fileContent])

  const lineRef = useRef<HTMLDivElement[]>([])
  // @ts-ignore
  const handleLineNumberClick = (lineIndex) => {
    setShowEditor(!showEditor)
    const codeLineNumber = lineRef.current[lineIndex]

    if (showEditor) {
      const editorContainer = document.createElement('div')

      editorContainer.className = 'editor-container'

      const root = createRoot(editorContainer)

      root.render(<Editor />)
      if (codeLineNumber && codeLineNumber.parentNode) {
        codeLineNumber.parentNode.insertBefore(editorContainer, codeLineNumber.nextSibling)
      }
    } else {
      const editorContainer = document.querySelector('.editor-container')

      if (editorContainer && editorContainer.parentNode) {
        editorContainer.parentNode.removeChild(editorContainer)
      }
    }
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
      <div className={styles.viewChangeTab}>
        <button className={styles.viewChangeTabButton}>Code</button>
        <button className={styles.viewChangeTabButton}>Blame</button>
      </div>

      <Highlight theme={themes.github} code={fileContent} language='rust'>
        {({ style, tokens, getLineProps, getTokenProps }) => (
          <pre
            style={{
              ...style,
              padding: '16px',
              paddingTop: '70px'
            }}
            className='overflow-x-auto whitespace-pre rounded-lg bg-gray-100 p-4 text-sm'
          >
            {!lfs &&
              tokens.map((line, i) => (
                <div
                  key={i}
                  {...getLineProps({ line })}
                  ref={(el) => lineRef.current[i] = el as HTMLDivElement}
                >
                  <button
                    onClick={() => handleLineNumberClick(i)}
                    className={styles.lineNumberButton}
                    style={{
                      marginLeft: '8px',
                      backgroundColor: 'rgb(247, 237, 224, 0.7)',
                      width: '25px',
                      height: '17px',
                      lineHeight: '17px',
                      borderRadius: '3px',
                      marginTop: '5px',
                      border: 'none'
                    }}
                  >
                    +
                  </button>
                  <span className={styles.codeLineNumber}>{i + 1}</span>
                  {line.map((token, key) => (
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
