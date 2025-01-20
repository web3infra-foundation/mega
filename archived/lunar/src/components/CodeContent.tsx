// import Editor from './editor/Editor'
import 'github-markdown-css/github-markdown-light.css'
import { Highlight, themes } from "prism-react-renderer"
import { useState } from 'react'
import { createRoot } from 'react-dom/client'
import styles from './CodeContent.module.css'

const CodeContent = ({ fileContent }) => {

    const [showEditor, setShowEditor] = useState(false);

    const handleLineNumberClick = (lineIndex) => {
        setShowEditor(!showEditor);
        const lineNumberButton = document.getElementsByClassName('codeLineNumber')[lineIndex];
        const codeLineNumber = lineNumberButton.closest('.token-line');
        if (showEditor) {
            const editorContainer = document.createElement('div');
            editorContainer.className = 'editor-container';

            // render the Editor into the container
            const root = createRoot(editorContainer);
            // root.render(<Editor />)
            if (codeLineNumber && codeLineNumber.parentNode) {
                codeLineNumber.parentNode.insertBefore(editorContainer, codeLineNumber.nextSibling);

            }
        } else {
            const editorContainer = document.querySelector('.editor-container');
            if (editorContainer && editorContainer.parentNode) {
                editorContainer.parentNode.removeChild(editorContainer);
            }
        }

    };

    return (
        <div className={styles.fileCodeContainer} >
            <div className={styles.viewChangeTab}>
                <button className={styles.viewChangeTabButton}>
                    Code
                </button>
                <button className={styles.viewChangeTabButton}>
                    Blame
                </button>
            </div>

            <Highlight
                theme={themes.github}
                code={fileContent}
                language="rust"
            >
                {({ className, style, tokens, getLineProps, getTokenProps }) => (
                    <pre style={style} className={styles.codeShowContainer}>
                        {tokens.map((line, i) => (
                            <div key={i} {...getLineProps({ line })}>
                                <button onClick={(event) => handleLineNumberClick(i)} className={styles.lineNumberButton} style={{ marginLeft: '8px', backgroundColor: 'rgb(247, 237, 224, 0.7)', width: '25px', height: '17px', lineHeight: '17px', borderRadius: '3px', marginTop: '5px', border: 'none' }}>+</button>
                                <span className={styles.codeLineNumber}>{i + 1}</span>
                                {line.map((token, key) => (
                                    <span key={key} {...getTokenProps({ token })} />
                                ))}
                            </div>
                        ))}
                    </pre>
                )}
            </Highlight>
        </div>
    )

}

export default CodeContent;
