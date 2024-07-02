
import 'github-markdown-css/github-markdown-light.css';
import { useRouter } from 'next/router';
import { useEffect, useState } from 'react';
import Markdown from 'react-markdown';

import { formatDistance, fromUnixTime } from 'date-fns'

const CodeTable = ({ directory, readmeContent }) => {

    const router = useRouter();
    const currentProjectDir = directory.data || [];
    // const [showEditor, setShowEditor] = useState(false);
    const [showTree, setShowTree] = useState(false);
    // const [treeData, setTreeData] = useState("");
    const [updateTree, setUpdateTree] = useState(false);
    // const [currentPath, setCurrentPath] = useState([]); // for breadcrumb
    // const { DirectoryTree } = Tree;
    // const [expandedKeys, setExpandedKeys] = useState([]);
    const fileCodeContainerStyle = showTree ? { width: '80%', marginLeft: '17%', borderRadius: '0.5rem', marginTop: '10px' } : { width: '90%', margin: '0 auto', borderRadius: '0.5rem', marginTop: '10px' };
    const dirShowTrStyle = { borderBottom: '1px solid  rgba(0, 0, 0, 0.1)', }

    useEffect(() => {
        if (updateTree) {
            setUpdateTree(false);
        }
    }, [updateTree]);

    const handleFileClick = (file) => {
        const current = router.query.path.join("/");
        const newPath = `/blob/${current}/${file.name}`;
        router.push(newPath);
        setShowTree(true);
    };

    const handleDirectoryClick = async (directory) => {
        const current = router.query.path;
        var newPath = '';
        if (current) {
            newPath = `/tree/${current.join("/")}/${directory.name}`;
        } else {
            newPath = `/tree/${directory.name}`;
        }
        router.push({
            pathname: newPath,
        });

        setShowTree(true);

        // try {
        //     const response = await fetch(`/api/tree?path=${encodeURIComponent(directory.path)}`);

        //     if (!response.ok) {
        //         throw new Error('Failed to fetch tree data');
        //     }

        //     const responseData = await response.json();
        //     console.log('Response data:', responseData);

        //     const subTreeData = convertToTreeData(responseData.data);
        //     const newTreeData = appendTreeData(treeData, subTreeData, directory.id);
        //     setTreeData(newTreeData);
        //     setUpdateTree(true);
        //     setExpandedKeys([...expandedKeys, directory.id]);
        //     setCurrentPath([...currentPath, directory.name]); // for breadcrumb
        //     console.log(newTreeData);
        //     console.log(treeData);
        // } catch (error) {
        //     console.error('Error fetching tree data:', error);
        // }
    };

    const handleGoBack = () => {
        router.back();
    };

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


    // sortProjectsByType function to sort projects by file type
    const sortProjectsByType = (projects) => {
        return projects.sort((a, b) => {
            if (a.content_type === 'directory' && b.content_type === 'file') {
                return -1; // directory comes before file
            } else if (a.content_type === 'file' && b.content_type === 'directory') {
                return 1; // file comes after directory
            } else {
                return 0; // maintain original order
            }
        });
    };



    // convert the dir to tree data
    const convertToTreeData = (responseData) => {
        // console.log("!!!!!!!!!!!!in convert");
        return sortProjectsByType(responseData).map(item => {
            const treeItem = {
                title: item.name,
                key: item.id,
                isLeaf: item.content_type !== 'directory',
                path: item.path,
                expanded: false, // initialize expanded state to false
                children: [] // eneure every node having the children element
            };
            return treeItem;
        });
    };

    // append the clicked dir to the treeData
    const appendTreeData = (treeData, subItems, clickedNodeKey) => {
        return treeData.map(item => {
            if (item.key === clickedNodeKey) {
                return {
                    ...item,
                    children: subItems
                };
            } else if (Array.isArray(item.children)) {
                return {
                    ...item,
                    children: appendTreeData(item.children, subItems, clickedNodeKey)
                };
            }
        });
    };



    return (
        <div className="dirTable" style={fileCodeContainerStyle}>
            <div className="innerTable">
                <table className="dirShowTable">
                    <thead className="dirShowTableThead">
                        <tr>
                            <th scope="col" className="dirShowTableTr">
                                Name
                            </th>
                            <th scope="col" className="dirShowTableTr">
                                Message
                            </th>
                            <th scope="col" className="dirShowTableTr">
                                Date
                            </th>
                        </tr>
                    </thead>
                    <tbody className="dirShowTableTbody">
                        {showTree && (
                            <tr style={dirShowTrStyle} className="dirShowTr" key="back">
                                <td className="projectName ">
                                    <img src="/icons/folder.svg" className='fileTableIcon' alt="File icon" />
                                    <span onClick={() => handleGoBack()}>..</span>
                                </td>
                                <td></td>
                                <td></td>
                            </tr>
                        )}

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
                                <td className="projectCommitMsg ">{project.message}</td>
                                <td className="projectCommitMsg">
                                    {formatDistance(fromUnixTime(project.date), new Date(), { addSuffix: true })}
                                </td>
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
    );
};



export default CodeTable;
