import { LexicalComposer } from '@lexical/react/LexicalComposer';
import ExampleTheme from './ExampleTheme';
import { ContentEditable } from '@lexical/react/LexicalContentEditable';
import { RichTextPlugin } from '@lexical/react/LexicalRichTextPlugin';
import { LexicalErrorBoundary } from '@lexical/react/LexicalErrorBoundary';

const LexicalContent = ({ lexicalJson }: { lexicalJson: string }) => {
    const editorConfig = {
        namespace: '',
        nodes: [],
        onError(error: Error) {
            throw error;
        },
        theme: ExampleTheme,
        editable: false,
        editorState: lexicalJson,
    };

    const placeholder = 'No description provided.';

    return (
        <LexicalComposer initialConfig={editorConfig}>
            <div className="editor-container" style={{border: "None", margin: "0px"}}>
                <div className="editor-inner" >
                    <RichTextPlugin
                        contentEditable={
                            <ContentEditable
                                className="editor-input"
                                style={{minHeight: '75px'}}
                                aria-placeholder={placeholder}
                                placeholder={
                                    <div className="editor-placeholder">{placeholder}</div>
                                }
                            />
                        }
                        ErrorBoundary={LexicalErrorBoundary}
                    />
                </div>
            </div>
        </LexicalComposer>
    );
};



export default LexicalContent;