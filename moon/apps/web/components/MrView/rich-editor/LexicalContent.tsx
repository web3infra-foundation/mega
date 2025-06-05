import { LexicalComposer } from '@lexical/react/LexicalComposer';
import ExampleTheme from './ExampleTheme';
import { ContentEditable } from '@lexical/react/LexicalContentEditable';
import { RichTextPlugin } from '@lexical/react/LexicalRichTextPlugin';
import { LexicalErrorBoundary } from '@lexical/react/LexicalErrorBoundary';
import type { LexicalEditor } from 'lexical';

const LexicalContent = ({ lexicalJson }: { lexicalJson: string }) => {
    const editorConfig = {
        namespace: '',
        nodes: [],
        onError(error: Error) {
            throw error;
        },
        theme: ExampleTheme,
        editable: false,
        editorState: (editor: LexicalEditor) => {
            if (lexicalJson) {
                try {
                    const parsedState = editor.parseEditorState(lexicalJson);
                    
                    editor.setEditorState(parsedState);
                } catch (e) {
                    // eslint-disable-next-line no-console
                    console.warn('Invalid lexical JSON, loading empty editor state.');
                }
            }
        },
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