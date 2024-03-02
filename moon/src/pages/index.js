import Editor from '@/components/editor/Editor';
import { Breadcrumb, Tree } from 'antd/lib';
import axios from 'axios';
import 'github-markdown-css/github-markdown-light.css';
import { useRouter } from 'next/router';
import { Highlight, themes } from "prism-react-renderer";
import { useState } from 'react';
import ReactDOM from 'react-dom';
import Markdown from 'react-markdown';
import Bottombar from '../components/Bottombar';
import TopNavbar from '../components/TopNavbar';
import '../styles/index.css';

const HomePage = ({ directory, readmeContent, fileContent }) => {
    console.log(directory);
    const router = useRouter();
    const currentProjectDir = directory.items || [];
    const [currentFileContent, setCurrentFileContent] = useState("");
    const [showEditor, setShowEditor] = useState(false);
    const [showTree, setShowTree] = useState(false);

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


    const handleFileClick = (file) => {
        console.log(fileContent);
        setShowTree(true);
        router.push(`/?object_id=${file.id}`);
    };

    const handleDirectoryClick = (directory) => {
        setShowTree(true);
        router.push(`/?repo_path=${directory.repo_path}&object_id=${directory.id}`);
    };

    const fileCodeContainerStyle = showTree ? { width: '80%', marginLeft: '17%', borderRadius: '0.5rem', marginTop: '10px' } : { width: '90%', margin: '0 auto', borderRadius: '0.5rem', marginTop: '10px', marginTop: '40px' };

    const dirShowTrStyle = { borderBottom: '1px solid  rgba(0, 0, 0, 0.1)', }

    // sort by file type, render folder type first
    const sortedProjects = currentProjectDir.sort((a, b) => {
        if (a.content_type === 'directory' && b.content_type === 'file') {
            return -1;
        } else if (a.content_type === 'file' && b.content_type === 'directory') {
            return 1;
        } else {
            return 0;
        }
    });


    const { DirectoryTree } = Tree;
    const onSelect = (keys, info) => {
        console.log('Trigger Select', keys, info);
    };
    const onExpand = (keys, { node }) => {
        console.log(node);
        router.push(`/?repo_path=${node.path}&object_id=${node.key}`);
        console.log('Trigger Expand', keys);
    };

    const convertToTreeData = (directory) => {
        return directory.map(item => {
            const treeItem = {
                title: item.name,
                key: item.id,
                isLeaf: item.content_type !== 'directory',
                path: item.path,
            };

            if (item.content_type === 'directory' && item.children) {
                treeItem.children = convertToTreeData(item.children);
            }

            return treeItem;
        });
    };

    const treeData = convertToTreeData(directory.items);
    const breadCrumbItems = [
        {
            title: 'Home',
        },
        {
            title: <a href="">Application Center</a>,
        },
        {
            title: <a href="">Application List</a>,
        },
        {
            title: 'An Application',
        },
    ]


    return (
        <div>
            <TopNavbar />
            {showTree && (
                <div className='dirTreeContainer'>
                    <DirectoryTree
                        multiple
                        onSelect={onSelect}
                        onExpand={onExpand}
                        treeData={treeData}
                    />
                </div>
            )}
            {showTree && (
                <Breadcrumb className='breadCrumb'
                    items={breadCrumbItems}
                />
            )}

            {!fileContent && (
                <div className="dirTable" style={fileCodeContainerStyle}>
                    <div className="innerTable">
                        <table className="dirShowTable">
                            <thead className="dirShowTableThead">
                                <tr>
                                    <th scope="col" className="dirShowTableTr">
                                        Name
                                    </th>
                                    <th scope="col" className="dirShowTableTr">
                                        commit
                                    </th>
                                    <th scope="col" className="dirShowTableTr">
                                        commitData
                                    </th>
                                </tr>
                            </thead>
                            <tbody className="dirShowTableTbody">
                                {sortedProjects.map((project) => (
                                    <tr style={dirShowTrStyle} className="dirShowTr" key={project.id}>
                                        {project.content_type === 'file' && (
                                            <td className="projectName ">
                                                <img src="/icons/file.svg" className='fileTableIcon' alt="File icon" />
                                                <span onClick={() => handleFileClick(project)}>{project.name}</span>
                                            </td>
                                        )}
                                        {project.content_type === 'directory' && (
                                            <td className="projectName ">
                                                <img src="/icons/folder.svg" className='fileTableIcon' alt="File icon" />
                                                <span onClick={() => handleDirectoryClick(project)}>{project.name}</span>
                                            </td>
                                        )}
                                        <td className="projectCommitMsg ">{project.commit_msg}</td>
                                        <td className="projectCommitMsg">{project.commit_date}</td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                    {readmeContent && (
                        <div className='markdownContent'>
                            <div className="markdown-body">
                                <Markdown>{readmeContent}</Markdown>
                            </div>
                        </div>
                    )}
                </div>
            )}

            {fileContent && (
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
            )}
            <Bottombar />
        </div>
    );
};



export async function getServerSideProps(context) {
    const MEGA_URL = 'http://localhost:8000';

    // get the parameters form context
    const { repo_path, object_id } = context.query;

    // obtain the current directory, the root directory only has the 'path' parameter without the 'id' parameter. Both parameters exist for non-root directories
    const response = repo_path && object_id
        ? await axios.get(`${MEGA_URL}/api/v1/tree?repo_path=/projects/freighter&object_id=${encodeURIComponent(object_id)}`)
        : await axios.get(`${MEGA_URL}/api/v1/tree?repo_path=/projects/freighter`);

    const directory = response.data;
    var readmeContent = '';
    var fileContent = '';

    // get the file content
    if (object_id) {
        try {
            const fileResponse = await axios.get(`${MEGA_URL}/api/v1/blob?object_id=${object_id}`, { withCredentials: true });
            fileContent = fileResponse.data.row_data;
        } catch (error) {
            console.error("Error fetching file content:", error);
        }
    }

    // get the readme file content
    for (const project of directory.items || []) {
        if (project.name === 'README.md' && project.content_type === 'file') {
            try {
                const response = await axios.get(`${MEGA_URL}/api/v1/blob?object_id=${project.id}`, { withCredentials: true });
                readmeContent = response.data.row_data;
                break;
            } catch (error) {
                console.error("Error fetching README content:", error);
            }

        }
    }
    return {
        props: {
            directory,
            readmeContent,
            fileContent,
        },
    };
}


export default HomePage;
