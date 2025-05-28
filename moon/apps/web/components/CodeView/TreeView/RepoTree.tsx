import 'github-markdown-css/github-markdown-light.css'
// import { DownloadIcon} from '@gitmono/ui/Icons'
import { DownOutlined } from '@ant-design/icons/lib'
import { useState, useEffect, useCallback } from 'react'
import { useRouter, usePathname  } from 'next/navigation'
import { Tree, TreeProps } from 'antd/lib'
import { DataNode, EventDataNode } from 'antd/lib/tree'

interface TreeNode extends DataNode {
  title: string;
  key: string;
  isLeaf: boolean;
  path: string;
  children?: TreeNode[];
}

const RepoTree = ({ directory }:any) => {
    const router = useRouter();
    const pathname = usePathname();
    const [treeData, setTreeData] = useState<TreeNode[]>([]);
    const [updateTree, setUpdateTree] = useState(false);
    const [expandedKeys, setExpandedKeys] = useState<string[]>([]);

    const convertToTreeData = useCallback((directory: any) => {
        return sortProjectsByType(directory)?.map((item) => {
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
        setTreeData(convertToTreeData(directory));
    }, [directory, convertToTreeData]);


    useEffect(() => {
        if (updateTree) {
            setUpdateTree(false);
        }
    }, [updateTree]);

    // sortProjectsByType function to sort projects by file type
    const sortProjectsByType = (projects: any[]) => {
        return projects?.sort((a, b) => {
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
    const appendTreeData = (treeData:any, subItems:object, clickedNodeTitle: string) => {
        return treeData.map((item: { title: string; children: any }) => {
            if (item.title === clickedNodeTitle) {
                return {
                    ...item,
                    children: subItems
                };
            } else if (Array.isArray(item.children)) {
                return {
                    ...item,
                    children: appendTreeData(item.children, subItems, clickedNodeTitle)
                };
            }
        });
    };

    const onExpand = async (expandedKeys: any[], {expanded, node}: any) => {
        if (expanded) {
            let responseData;

            try {
                // query tree by path
                const reqPath = pathname?.replace('/tree', '') + '/' + node.title;

                if (node.path && node.path !== '' && node.path !== undefined) {
                    responseData = await fetch(`/api/tree?path=${node.path}`)
                      .then(response => response.json())
                      .catch(e => {
                          throw new Error('Failed to fetch tree data');
                      })
                } else {
                    responseData = await fetch(`/api/tree?path=${reqPath}`)
                      .then(response => response.json())
                      .catch(e => {
                          throw new Error('Failed to fetch tree data');
                      })
                }
            } catch (error) {
                // eslint-disable-next-line no-console
                console.error('Error fetching tree data:', error);

            }
            const subTreeData = convertToTreeData(responseData?.data?.data);
            const newTreeData = appendTreeData(treeData, subTreeData, node.title);

            setExpandedKeys([...expandedKeys, node.key]);
            setTreeData(newTreeData);
        } else {
            setExpandedKeys(expandedKeys.filter(key => key !== node.key));
        }
    };

    const onSelect: TreeProps<TreeNode>['onSelect'] = (
      selectedKeys: React.Key[],
      _info: {
        event: 'select';
        selected: boolean;
        node: EventDataNode<TreeNode>;
        selectedNodes: TreeNode[];
        nativeEvent: MouseEvent;
      }
    ) => {
        const selectedKey = selectedKeys[0]?.toString();
        // only click one, example: click the first one is ['0-0'], then the array index is 0
        const pathArray = selectedKey.split('-').map((part: string) => parseInt(part, 10));
        // according to the current route, splicing the next route and determine the type to jump
        const real_path = pathname?.replace('/tree', '');

        if (Array.isArray(treeData) && treeData?.length > 0) {
            if (Array.isArray(pathArray) && pathArray.length === 2) {
                // root folder
                const clickNode = treeData[pathArray[1]] as TreeNode
                // determine file type and router push

                if (clickNode.isLeaf) {
                    router.push(`/blob/${real_path}/${clickNode.title}`);
                } else {
                    router.push(`${pathname}/${clickNode.title}`);
                }
            } else {
                // child list, recursively find the target node
                const findNode = (data: TreeNode[], indices: number[]): TreeNode | null => {
                    if (indices.length === 0) return null;
                    if (indices.length === 1) return data[indices[0]];

                    const node = data[indices[1]] as TreeNode;
                    let current = node;
                    
                    for (let i = 2; i < indices.length; i++) {
                        if (!current.children) return null;
                        current = current.children[indices[i]] as TreeNode;
                    }
                    
                    return current;
                };

                // build the path
                const buildPath = (indices: number[]): string => {
                    let path = '';
                    let current = treeData[indices[1]] as TreeNode;

                    path += current.title;
                    
                    for (let i = 2; i < indices.length; i++) {
                        if (!current.children) break;
                        current = current.children[indices[i]] as TreeNode;
                        path += '/' + current.title;
                    }
                    
                    return path;
                };

                const targetNode = findNode(treeData, pathArray);

                if (targetNode) {
                    const fullPath = buildPath(pathArray);

                    if (targetNode.isLeaf) {
                        router.push(`/blob/${real_path}/${fullPath}`);
                    } else {
                        router.push(`${pathname}/${fullPath}`);
                    }
                }
            }
        } else {
            router.push(`${pathname}`)
        }
    };

    return (
        <div className="mt-4">
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
