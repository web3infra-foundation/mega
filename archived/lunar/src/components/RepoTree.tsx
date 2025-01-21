import 'github-markdown-css/github-markdown-light.css'
import { DownOutlined } from '@ant-design/icons/lib'
import { useState, useEffect, useCallback } from 'react'
import { useRouter } from 'next/navigation'
import { Tree } from 'antd/lib'
import styles from './RepoTree.module.css'

const RepoTree = ({ directory }) => {
    const router = useRouter();
    const [treeData, setTreeData] = useState();
    const [updateTree, setUpdateTree] = useState(false);
    const [expandedKeys, setExpandedKeys] = useState([]);

    const convertToTreeData = useCallback((directory) => {
        return sortProjectsByType(directory).map(item => {
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
    }, []);

    useEffect(() => {
        let data = convertToTreeData(directory);
        setTreeData(data);
    }, [directory, convertToTreeData]);


    useEffect(() => {
        if (updateTree) {
            setUpdateTree(false);
        }
    }, [updateTree]);





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

    const onExpand = async (keys, { expanded, node }) => {
        // push new url and query to router
        console.log("OnExpanded!");
        console.log("keys", keys);
        console.log("node", node.path);
        // router.push({ query: { repo_path: "/projects/freighter", object_id: node.key } });
        var responseData;
        try {
            const response = await fetch(`/api/tree?path=${node.path}`);

            if (!response.ok) {
                throw new Error('Failed to fetch tree data');
            }

            console.log('Response status:', response.status);

            responseData = await response.json();
            console.log('Response data:', responseData);

        } catch (error) {
            console.error('Error fetching tree data:', error);
        }
        // onRenderTree(node.key);
        if (expanded) {
            const subTreeData = convertToTreeData(responseData.items);
            const newTreeData = appendTreeData(treeData, subTreeData, node.key);
            // setExpandedKeys([...expandedKeys, node.key]);
            setTreeData(newTreeData);
            // setCurrentPath([...currentPath, node.title]); // for breadcrumb
        } else {
            setExpandedKeys(expandedKeys.filter(key => key !== node.key));
        }
    };

    const onSelect = (keys, info) => {
        router.push(`/?object_id=${keys}`);
        console.log('Trigger Select', keys, info);
    };

    return (
        <div className={styles.dirTreeContainer}>
            <Tree
                // multiple
                onSelect={onSelect}
                onExpand={onExpand}
                treeData={treeData}
                showLine={true}
                switcherIcon={<DownOutlined />}
                expandedKeys={expandedKeys}
            />
        </div >
    );
};

export default RepoTree;
