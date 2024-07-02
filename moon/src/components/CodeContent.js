import Editor from '@/components/editor/Editor';
import 'github-markdown-css/github-markdown-light.css';
import { Highlight, themes } from "prism-react-renderer";
import ReactDOM from 'react-dom';

const CodeContent = ({ fileContent }) => {

    const handleLineNumberClick = (lineIndex) => {
        setShowEditor(!showEditor);
        const lineNumberButton = document.getElementsByClassName('codeLineNumber')[lineIndex];
        const codeLineNumber = lineNumberButton.closest('.token-line');
        if (showEditor) {
            const editorContainer = document.createElement('div');
            editorContainer.className = 'editor-container';

            // render the Editor into the container
            ReactDOM.render(<Editor />, editorContainer);
            codeLineNumber.parentNode.insertBefore(editorContainer, codeLineNumber.nextSibling);
        } else {
            const editorContainer = document.querySelector('.editor-container');
            if (editorContainer) {
                editorContainer.parentNode.removeChild(editorContainer);
            }
        }

    };


    return (
        <div className="fileCodeContainer">
            <div className="viewChangeTab">
                <button className='viewChangeTabButton'>
                    Code
                </button>
                <button className='viewChangeTabButton'>
                    Blame
                </button>
            </div>

            <Highlight
                theme={themes.github}
                code={fileContent}
                language="rust"
            >
                {({ className, style, tokens, getLineProps, getTokenProps }) => (
                    <pre style={style} className="codeShowContainer">
                        {tokens.map((line, i) => (
                            <div key={i} {...getLineProps({ line })}>
                                <button onClick={(event) => handleLineNumberClick(i)} className="lineNumberButton" style={{ marginLeft: '8px', backgroundColor: 'rgb(247, 237, 224, 0.7)', width: '25px', height: '17px', lineHeight: '17px', borderRadius: '3px', marginTop: '5px', border: 'none' }}>+</button>
                                <span className="codeLineNumber">{i + 1}</span>
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
