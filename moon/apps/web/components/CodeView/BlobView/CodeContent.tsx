import 'github-markdown-css/github-markdown-light.css'
import { useEffect, useRef, useState } from 'react'
import { Highlight, themes } from 'prism-react-renderer'

import styles from './CodeContent.module.css'

// @ts-ignore
const CodeContent = ({ fileContent }) => {
  const [lfs, setLfs] = useState(false)

  useEffect(() => {
    if (isLfsContent(fileContent)) {
      setLfs(true)
    }
  }, [fileContent])

  const lineRef = useRef<HTMLDivElement[]>([])

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
      <Highlight theme={themes.github} code={fileContent} language='rust'>
        {({ style, tokens, getLineProps, getTokenProps }) => (
          <pre
            style={{
              ...style,
              padding: '16px',
              paddingTop: '0px'
            }}
            className='overflow-x-auto whitespace-pre rounded-lg bg-gray-100 p-4 text-sm'
          >
            {!lfs &&
              tokens.map((line, i) => (
                <div
                  /* eslint-disable-next-line react/no-array-index-key */
                  key={i}
                  {...getLineProps({ line })}
                  // @ts-ignore
                  ref={(el) => lineRef.current[i] = el as HTMLDivElement}
                >
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

export default CodeContent;