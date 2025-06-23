import * as React from 'react';
import { useCallback, useEffect, useState } from 'react';
import ArticleIcon from '@mui/icons-material/Article';
import FolderRounded from '@mui/icons-material/FolderRounded';
import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import { Box, CircularProgress } from '@mui/material';
import {
  TreeItemDragAndDropOverlay,
  TreeItemIcon,
  TreeItemProvider,
  useTreeItem,
  useTreeItemModel,
  UseTreeItemParameters,
} from '@mui/x-tree-view';
import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import {
  TreeItemContent,
  TreeItemGroupTransition,
  TreeItemIconContainer,
  TreeItemLabel,
  TreeItemRoot,
} from '@mui/x-tree-view/TreeItem';
import { usePathname } from 'next/navigation';
import { useRouter } from 'next/router';
import { styled, alpha } from '@mui/material/styles';
import { useGetTree } from '@/hooks/useGetTree';
import { v4 as uuidv4 } from 'uuid';

interface MuiTreeNode {
  id?: string;
  label?: string;
  path?: string;
  content_type?: FileType;
  isLeaf?: boolean;
  children?: MuiTreeNode[];
}

type FileType = 'file' | 'directory';

interface ExtendedTreeItemProps {
  content_type?: FileType;
  id: string;
  label: string;
}

interface CustomLabelProps {
  children: React.ReactNode;
  icon?: React.ElementType;
  expandable?: boolean;
}

function CustomLabel({ icon: Icon, children, ...other }: CustomLabelProps) {
  return (
    <TreeItemLabel
      {...other}
      sx={{
        display: 'flex',
        alignItems: 'center',
      }}
    >
      {Icon && (
        <Box component={Icon} className="labelIcon" color="inherit" sx={{ mr: 1, fontSize: '1.2rem' }} />
      )}
      <TreeItemLabel sx={{fontSize: '14px'}}>{children}</TreeItemLabel>
    </TreeItemLabel>
  );
}

const getIconFromFileType = (fileType: FileType, isExpanded:Boolean) => {
  switch (fileType) {
    case 'file':
      return ArticleIcon;
    case 'directory':
      return isExpanded ? FolderOpenIcon :FolderRounded;
    default:
      return ArticleIcon;
  }
};

interface CustomTreeItemProps
  extends Omit<UseTreeItemParameters, 'rootRef'>,
    Omit<React.HTMLAttributes<HTMLLIElement>, 'onFocus'> {
  }

const CustomTreeItem = React.forwardRef(function CustomTreeItem(
  { ...props }: CustomTreeItemProps,
  ref: React.Ref<HTMLLIElement>,
) {
  const { id, itemId, label, disabled, children, ...other } = props;
  const {
    getContextProviderProps,
    getRootProps,
    getContentProps,
    getIconContainerProps,
    getLabelProps,
    getGroupTransitionProps,
    getDragAndDropOverlayProps,
    status,
  } = useTreeItem({ id, itemId, children, label, disabled, rootRef: ref });

  const item = useTreeItemModel<ExtendedTreeItemProps>(itemId)!;
  let icon;

  if (status.expandable) {
    icon = getIconFromFileType(item.content_type || 'file', status.expanded);
  } else if (item.content_type) {
    icon = getIconFromFileType(item.content_type, false);
  }

  const StyledGroupTransition = styled(TreeItemGroupTransition)(({ theme }) => ({
    marginLeft: 15,
    borderLeft: `1px dashed ${alpha(theme.palette.text.primary, 0.4)}`,
  }));

  return (
    <TreeItemProvider {...getContextProviderProps()}>
      <TreeItemRoot {...getRootProps(other)}>
        {item.label === '加载中...' ? ( 
          <TreeItemContent {...getContentProps()} sx={{ paddingLeft: 2 }}>
            <TreeItemIconContainer {...getIconContainerProps()}>
              <CircularProgress size="1.2rem" color="inherit" />
            </TreeItemIconContainer>
          </TreeItemContent>
        ) : (
          <TreeItemContent {...getContentProps()} sx={{ paddingLeft: 1 }}>
            <TreeItemIconContainer {...getIconContainerProps()}>
              <TreeItemIcon status={status} />
            </TreeItemIconContainer>
            
            {/* label */}
            <CustomLabel
              {...getLabelProps({
                icon,
                expandable: status.expandable && status.expanded, 
              })}
            />
            <TreeItemDragAndDropOverlay {...getDragAndDropOverlayProps()} />
          </TreeItemContent>
        )}
        {children && <StyledGroupTransition {...getGroupTransitionProps()} />}
      </TreeItemRoot>
    </TreeItemProvider>
  );
});

const RepoTree = ({ directory }: { directory: any[] }) => {
  const router = useRouter();
  const scope = router.query.org as string;
  const pathname = usePathname();
  const basePath = pathname?.replace(`/${scope}/code/tree`, ''); 

  const [treeData, setTreeData] = useState<MuiTreeNode[]>([]); 

  const [expandedNodes, setExpandedNodes] = useState<string[]>([]); 
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  const [loadPath, setLoadPath] = useState<string | null >(null);
  const [targetNodeId, setTargetNodeId] = useState<string | null >(null);
  // const [loadingError, setLoadingError] = useState<string| null>(null);
  const [isInitialLoading, setIsInitialLoading] = useState(true); 
  const { data: response, isLoading, error } = useGetTree({ path: loadPath??undefined});

  const convertToTreeData = useCallback((parentBasePath: string, directory: any[]) => {
    if (!Array.isArray(directory) || directory.length === 0) {
      // console.warn('convertToTreeData: 接收到非数组或空数据', directory);
      return [];
    }
    
    return sortProjectsByType(directory).map((item) => {
      
      const currentPath = `${parentBasePath}/${item.name}`.replace('//', '/') || '/';
      // console.log('生成节点路径:', item.name, '=>', currentPath);

      return {
        id: uuidv4(), 
        label: item.name,
        path: currentPath,
        isLeaf: item.content_type !== 'directory',
        content_type: item.content_type,
        children: item.content_type === 'directory' ? [
          {
            id: uuidv4(), 
            label: '加载中...', 
            isLeaf: true,
          },
        ] : undefined,
      };
    });
  }, []); 

  useEffect(() => {
    if (!Array.isArray(directory) || directory.length === 0) {
      // console.warn('RepoTree: 接收到非数组或空的初始数据', directory);
      setIsInitialLoading(false);
      return;
    }
    
    const rootPath = basePath || '/'; // 使用基路径作为根路径
    
    setTreeData(convertToTreeData(rootPath, directory));
    setIsInitialLoading(false);
  }, [directory, convertToTreeData,basePath]);

  const sortProjectsByType = (projects: any[]) => {
    if (!Array.isArray(projects) || projects.length === 0) {
      // console.warn('sortProjectsByType: 接收到非数组或空数据', projects);
      return [];
    }
    
    return projects.sort((a, b) => {
      if (a.content_type === 'directory' && b.content_type === 'file') {
        return -1;
      } else if (a.content_type === 'file' && b.content_type === 'directory') {
        return 1;
      } else {
        return 0;
      }
    });
  };

  const updateTreeData = useCallback(
    (currentTree: MuiTreeNode[], nodeId: string, newChildren: MuiTreeNode[]): any => {
      if (!Array.isArray(currentTree) || currentTree.length === 0) {
        // console.warn('updateTreeData: 接收到非数组或空的当前树数据', currentTree);
        return [];
      }
      
      return currentTree.map((node) => {
        if (node.id === nodeId) {
          return {
            ...node,
            children: newChildren,
            isLeaf: newChildren.length === 0,
          };
        }
        if (node.children && node.children.length > 0) {
          return {
            ...node,
            children: updateTreeData(node.children, nodeId, newChildren),
          };
        }
        return node;
      });
    },
    [],
  );

  const findNode = useCallback(
    (data: MuiTreeNode[], nodeId: string): MuiTreeNode | null => {
      if (!Array.isArray(data) || data.length === 0) {
        return null;
      }
      
      for (const node of data) {

        if (node.id === nodeId) return node;

        if (node.children) {
          const found = findNode(node.children, nodeId);

          if (found) return found;
        }
      }
      return null;
    },
    [],
  );

  const handleNodeToggle = useCallback(
    (_event: React.SyntheticEvent<Element, Event> | null, nodeIds: string[]) => {
      const newlyExpandedId = nodeIds.find((id) => !expandedNodes.includes(id));
      
      if (newlyExpandedId) {
        const targetNode = findNode(treeData, newlyExpandedId);

          const reqPath = targetNode?.path?.startsWith('/') 
          ? targetNode.path 
          : `/${targetNode?.path}`;

        if (
          targetNode && 
          targetNode.content_type === 'directory' && 
          targetNode.children?.[0]?.label === '加载中...' // 中文判断
        ) {
          setLoadPath(reqPath);
          setTargetNodeId(newlyExpandedId);
          // setLoadingError(null);
        }
      }
      
      setExpandedNodes(nodeIds);
    },
    [expandedNodes, treeData, setLoadPath, setTargetNodeId, findNode],
  );

  useEffect(() => {
    if (!loadPath) return;
    
    if (response && !isLoading) {
      if (
        !response.req_result || 
        !Array.isArray(response.data) || 
        response.data.length === 0
      ) {
        // console.error('API数据格式错误:', response);
        // setLoadingError('数据格式错误或为空');
        return;
      }
  
      const items = response.data;
      const newChildren = convertToTreeData(loadPath, items);
      const newTree = updateTreeData(treeData, targetNodeId!, newChildren);

      setTreeData(newTree);
      setLoadPath(null);
      setTargetNodeId(null);
    }
  
    // if (error) {
    //   console.error('Error fetching data:', error);
    //   // setLoadingError(error.message || '加载失败');
    // }
  }, [response, isLoading, error, loadPath, targetNodeId, treeData, convertToTreeData, updateTreeData]);


  const handleNodeSelect = useCallback(
    (_event: React.SyntheticEvent<Element, Event> | null, nodeId: string | null) => {
      if (!nodeId) return;

      setSelectedNode(nodeId);
      const node = findNode(treeData, nodeId);

      if (!node) return;

      const basePath = pathname?.replace(`/${scope}/code/tree`, '') || '';
      
      let fullPath = node.path;

      if (basePath && fullPath?.startsWith(basePath)) {
        fullPath = fullPath?.substring(basePath.length);
      }

      fullPath = fullPath?.replace(/^\//, '');
      if (node?.isLeaf) {
        router.push(`/${scope}/code/blob${basePath}/${fullPath}`);
      }
  
      },
    [pathname, router, treeData, scope, findNode],
  );

  return (
    <>
      {isInitialLoading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', padding: '16px' }}>
          <CircularProgress />
        </Box>
      ) : treeData.length === 0 ? (
        <Box sx={{ color: 'text.secondary', padding: '16px' }}>
          no data
        </Box>
      ) : (
        <RichTreeView
          items={treeData}
          defaultExpandedItems={['grid', 'pickers']}
          expandedItems={expandedNodes}
          selectedItems={selectedNode}
          onExpandedItemsChange={handleNodeToggle}
          onSelectedItemsChange={handleNodeSelect}
          sx={{ height: 'fit-content', flexGrow: 1, maxWidth: 400, overflowY: 'auto' }}
          slots={{
            item: (itemProps) => (
              <CustomTreeItem 
                {...itemProps}
              />
            )
          }}
        >
        </RichTreeView>
      )}
      {/* {loadingError && (
        <Box sx={{ color: 'error.main', mt: 2, padding: '0 16px' }}>
          加载错误: {loadingError}
        </Box>
      )} */}
    </>
  );
};

export default RepoTree;