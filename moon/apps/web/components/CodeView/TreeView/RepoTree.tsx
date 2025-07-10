import * as React from 'react';
import { useCallback, useEffect, useState, useRef } from 'react';
import ArticleIcon from '@mui/icons-material/Article';
import FolderRounded from '@mui/icons-material/FolderRounded';
import FolderOpenIcon from '@mui/icons-material/FolderOpen';
import { Box, Skeleton } from '@mui/material';
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
  id: string;
  label: string;
  path: string;
  content_type?: 'file' | 'directory';
  isLeaf?: boolean;
  children?: MuiTreeNode[];
  hasChildrenLoaded?: boolean;
  isPlaceholder?: boolean;
}

interface CustomLabelProps {
  children: React.ReactNode;
  icon?: React.ElementType;
  expandable?: boolean;
  onClick?: (event: React.MouseEvent) => void;
}

function CustomLabel({ icon: Icon, children, onClick, ...other }: CustomLabelProps) {
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
      <TreeItemLabel 
        sx={{fontSize: '14px', cursor: 'pointer'}} 
        onClick={(e) => {
          e.stopPropagation();
          onClick?.(e);
        }}
      >
        {children}
      </TreeItemLabel>
    </TreeItemLabel>
  );
}

const getIconFromFileType = (fileType: 'file' | 'directory' | undefined, isExpanded: boolean) => {
  switch (fileType) {
    case 'file':
      return ArticleIcon;
    case 'directory':
      return isExpanded ? FolderOpenIcon : FolderRounded;
    default:
      return ArticleIcon;
  }
};

interface CustomTreeItemProps
  extends Omit<UseTreeItemParameters, 'rootRef'>,
    Omit<React.HTMLAttributes<HTMLLIElement>, 'onFocus'> {
  onLabelClick?: (path: string, isDirectory: boolean) => void;
}

const CustomTreeItem = React.forwardRef(function CustomTreeItem(
  { onLabelClick, ...props }: CustomTreeItemProps,
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

  const item = useTreeItemModel<MuiTreeNode>(itemId)!;
  
  // 如果是占位节点，不渲染任何内容
  if (item.isPlaceholder) {
    return null;
  }

  let icon;

  if (item.content_type === 'directory') {
    icon = getIconFromFileType(item.content_type, status.expanded);
  } else {
    icon = getIconFromFileType(item.content_type, false);
  }

  const StyledGroupTransition = styled(TreeItemGroupTransition)(({ theme }) => ({
    marginLeft: 15,
    borderLeft: `1px dashed ${alpha(theme.palette.text.primary, 0.4)}`,
  }));

  return (
    <TreeItemProvider {...getContextProviderProps()}>
      <TreeItemRoot {...getRootProps(other)}>
        <TreeItemContent {...getContentProps()} sx={{ paddingLeft: 1 }}>
          <TreeItemIconContainer {...getIconContainerProps()}>
            <TreeItemIcon status={status} />
          </TreeItemIconContainer>
          
          <CustomLabel
            {...getLabelProps({
              icon,
            })}
            onClick={() => {
              if (item.content_type) {
                onLabelClick?.(item.path, item.content_type === 'directory');
              }
            }}
          />
          <TreeItemDragAndDropOverlay {...getDragAndDropOverlayProps()} />
        </TreeItemContent>
        {children && <StyledGroupTransition {...getGroupTransitionProps()} />}
      </TreeItemRoot>
    </TreeItemProvider>
  );
});

const RepoTree = ({ flag, onCommitInfoChange }: { flag: string, onCommitInfoChange?:Function }) => {
  const router = useRouter();
  const scope = router.query.org as string;
  const pathname = usePathname();
  const basePath = pathname?.replace(
    new RegExp(`\\/${scope}\\/code\\/(tree|blob)`), 
    ''
  ) || '/'; 

  const [treeData, setTreeData] = useState<MuiTreeNode[]>([]); 
  const [expandedNodes, setExpandedNodes] = useState<string[]>([]); 
  const [selectedNode, setSelectedNode] = useState<string | null>(null);
  const [isInitialLoading, setIsInitialLoading] = useState(true); 
  
  // 使用单个状态管理加载路径和节点
  const [loadingState, setLoadingState] = useState<{
    path: string;
    nodeId: string;
    isRefreshing: boolean;
  } | null>(null);
  
  const { data: treeItems, isLoading } = useGetTree({ 
    path: loadingState?.path || basePath
  });
  
  const currentPathNodeId = useRef<string | null>(null);
  const hasSetInitialExpansion = useRef(false);
  const loadedNodes = useRef<Set<string>>(new Set());

  // 创建占位节点函数
  const createPlaceholderNode = (): MuiTreeNode => ({
    id: `placeholder-${uuidv4()}`,
    label: "placeholder",
    path: "",
    isLeaf: true,
    isPlaceholder: true
  });

  // 排序函数：目录在前，文件在后
  const sortProjectsByType = useCallback((projects: MuiTreeNode[]): MuiTreeNode[] => {
    if (!Array.isArray(projects) || projects.length === 0) {
      return [];
    }
    
    return [...projects].sort((a, b) => {
      if (a.content_type === 'directory' && b.content_type === 'file') {
        return -1;
      } else if (a.content_type === 'file' && b.content_type === 'directory') {
        return 1;
      } else {
        return a.label.localeCompare(b.label);
      }
    });
  }, []);

  // 处理树组件的加载数据
  const convertToTreeData = useCallback((responseData: any): MuiTreeNode[] => {
    
    if (!responseData?.data) {
      return [];
    }

    const data = responseData.data;
    
    // 1. 创建路径到节点的映射
    const pathToNodeMap = new Map<string, MuiTreeNode>();
    
    // 2. 处理 file_tree 中的所有路径
    if (data.file_tree) {
      // 按路径长度排序，确保父节点先创建
      const sortedPaths = Object.keys(data.file_tree).sort((a, b) => a.split('/').length - b.split('/').length);
      
      sortedPaths.forEach(path => {
        const pathData = data.file_tree[path];
        
        if (!pathData) return;
        
        // 创建当前路径的节点（如果不存在）
        let currentNode = pathToNodeMap.get(path);

        if (!currentNode) {
          const label = path === '/' ? 'Repository' : path.split('/').pop() || '';

          currentNode = {
            id: path,
            label: label,
            path: path,
            content_type: 'directory',
            children: [],
            hasChildrenLoaded: path === basePath
          };
          pathToNodeMap.set(path, currentNode);
        }
        
        // 处理当前路径下的所有子项
        pathData.tree_items.forEach((item: any) => {
          const childPath = item.path;
          
          // 创建子节点（如果不存在）
          let childNode = pathToNodeMap.get(childPath);

          if (!childNode) {

            const isDirectory = item.content_type === 'directory';

            childNode = {
              id: childPath,
              label: item.name,
              path: childPath,
              content_type: item.content_type,
              isLeaf: !isDirectory,
              children: isDirectory ? [createPlaceholderNode()] : undefined,
              hasChildrenLoaded: false
            };
            pathToNodeMap.set(childPath, childNode);
          }
          
          // 确保子节点被添加到当前节点的children中
          if (currentNode.children && !currentNode.children.some(child => child.path === childPath)) {
            currentNode.children.push(childNode);
          }
        });
        
        // 排序当前节点的子节点
        if (currentNode.children) {
          currentNode.children = sortProjectsByType(currentNode.children);
        }
      });
    }
    
    // 3. 处理当前路径的 tree_items（覆盖file_tree中的数据）
    if (data.tree_items) {
      data.tree_items.forEach((item: any) => {
        const itemPath = item.path;
        const parentPath = itemPath.substring(0, itemPath.lastIndexOf('/')) || '/';
        
        // 获取父节点
        const parentNode = pathToNodeMap.get(parentPath);

        if (!parentNode) return;
        
        // 创建或更新当前节点
        let currentNode = pathToNodeMap.get(itemPath);

        if (!currentNode) {

          const isDirectory = item.content_type === 'directory';

          currentNode = {
            id: itemPath,
            label: item.name,
            path: itemPath,
            content_type: item.content_type,
            isLeaf: !isDirectory,
            children: isDirectory ? [createPlaceholderNode()] : undefined,
            hasChildrenLoaded: false
          };

          pathToNodeMap.set(itemPath, currentNode);

        } else {
          // 更新现有节点
          currentNode.label = item.name;
          currentNode.content_type = item.content_type;
          currentNode.isLeaf = item.content_type !== 'directory';
        }
        
        // 确保当前节点被添加到父节点
        if (parentNode.children && !parentNode.children.some(child => child.path === itemPath)) {
          parentNode.children.push(currentNode);
          parentNode.children = sortProjectsByType(parentNode.children);
        }
        
        // 标记当前路径节点为已加载
        if (itemPath === basePath) {
          currentNode.hasChildrenLoaded = true;
          currentPathNodeId.current = currentNode.id;
        }
      });
    }
    
    // 4. 获取根节点
    const rootNode = pathToNodeMap.get('/');

    if (!rootNode) return [];
    
    // 5. 返回根节点的子节点
    return rootNode.children ? sortProjectsByType(rootNode.children) : [];

  }, [basePath, sortProjectsByType]);

  // 在 useEffect 中使用
  useEffect(() => {

    if (!treeItems) return;
    // 获取完整的树结构
    const children = convertToTreeData(treeItems);

    // 设置树数据
    setTreeData(children);
    
    // 自动展开当前路径的节点
    if (currentPathNodeId.current && !hasSetInitialExpansion.current) {
      // 确保只设置一次初始展开
      setExpandedNodes(prev => {
        // 如果当前节点不在已展开列表中，则添加
        if (!prev.includes(currentPathNodeId.current!)) {
          return [...prev, currentPathNodeId.current!];
        }
        return prev;
      });
      
      // 标记已设置初始展开
      hasSetInitialExpansion.current = true;
    }
    
    setIsInitialLoading(false);
  }, [treeItems, convertToTreeData]);

  // 更新树中特定节点
  const updateTreeNode = useCallback((
    tree: MuiTreeNode[],
    nodeId: string,
    updateFn: (node: MuiTreeNode) => MuiTreeNode
  ): MuiTreeNode[] => {
    return tree.map(node => {
      if (node.id === nodeId) {
        return updateFn(node);
      }
      if (node.children && node.children.length > 0) {
        return {
          ...node,
          children: updateTreeNode(node.children, nodeId, updateFn)
        };
      }
      return node;
    });
  }, []);

  // 找到当前节点
  const findNode = useCallback(
    (data: MuiTreeNode[], nodeId: string): MuiTreeNode | null => {
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

  // 获取路径的所有父节点ID
  const getAllParentIds = useCallback((path: string): string[] => {
    if (!path || path === '/') return [];
    
    // 分割路径为各个部分
    const parts = path.split('/').filter(part => part !== '');
    
    // 生成所有父节点ID（包括自身）
    const parentIds: string[] = [];
    let currentPath = '';
    
    for (const part of parts) {
      currentPath = currentPath ? `${currentPath}/${part}` : `/${part}`;
      parentIds.push(currentPath);
    }
    
    return parentIds;
  }, []);

  // 监听 basePath 变化，自动展开路径
  useEffect(() => {

    if (basePath && !isInitialLoading && treeData.length > 0) {

      // 获取路径的所有父节点ID
      const parentIds = getAllParentIds(basePath);
      
      // 设置这些节点为展开状态
      setExpandedNodes(prev => {
        // 检查是否需要更新
        const hasNewNodes = parentIds.some(id => !prev.includes(id));

        if (!hasNewNodes) return prev;
        
        const newSet = new Set([...prev, ...parentIds]);

        return Array.from(newSet);
      });
      
      // 设置选中的节点
      setSelectedNode(basePath);
      
      // 确保路径中的所有节点都已加载
      parentIds.forEach(path => {
        const node = findNode(treeData, path);
        
        if (node && node.content_type === 'directory' && !loadedNodes.current.has(path)) {
          // 标记节点为正在加载
          loadedNodes.current.add(path);
          
          // 设置加载状态
          setLoadingState({
            path: node.path,
            nodeId: node.id,
            isRefreshing: false
          });
          
          // 设置节点加载状态
          setTreeData(prev => 
            updateTreeNode(prev, node.id, n => ({
              ...n,
              hasChildrenLoaded: false
            }))
          );
        }
      });
    }
  }, [basePath, isInitialLoading, treeData, findNode, updateTreeNode, getAllParentIds]);

  // 处理节点展开
  const handleNodeToggle = useCallback(
    (_event: React.SyntheticEvent<Element, Event> | null, nodeIds: string[]) => {
      // 更新所有展开状态
      setExpandedNodes(nodeIds);
      
      // 找出新展开的节点
      const newlyExpandedIds = nodeIds.filter(id => !expandedNodes.includes(id));
      
      // 处理所有新展开的节点
      newlyExpandedIds.forEach(nodeId => {
        const targetNode = findNode(treeData, nodeId);
        
        // 检查节点是否是目录
        if (targetNode && targetNode.content_type === 'directory' && !loadedNodes.current.has(nodeId)) {
          // 标记节点为正在加载
          loadedNodes.current.add(nodeId);
          
          // 设置加载状态
          setLoadingState({
            path: targetNode.path,
            nodeId: targetNode.id,
            isRefreshing: false
          });
          
          // 设置节点加载状态
          setTreeData(prev => 
            updateTreeNode(prev, targetNode.id, node => ({
              ...node,
              hasChildrenLoaded: false
            }))
          );
        }
      });
    },
    [expandedNodes, treeData, findNode, updateTreeNode]
  );

  // 处理子节点加载完成
  useEffect(() => {

    if (treeItems && !isLoading && loadingState) {

      const { nodeId } = loadingState;
      
      // 转换API数据为树节点
      const convertApiItems = (items: any[]): MuiTreeNode[] => {
        return items.map(item => {

          const isDirectory = item.content_type === 'directory';

          return {
            id: item.path,
            label: item.name,
            path: item.path,
            content_type: item.content_type,
            isLeaf: !isDirectory,
            children: isDirectory ? [createPlaceholderNode()] : undefined,
            hasChildrenLoaded: false
          };
        });
      };

      // 从API响应中提取直接子节点
      const newChildren = treeItems.data?.tree_items 
        ? convertApiItems(treeItems.data.tree_items) 
        : [];

      // 更新树数据 - 保留已存在的子节点
      setTreeData(prev => 
        updateTreeNode(prev, nodeId, node => {
          // 保留已存在的非占位子节点
          const existingChildren = (node.children || [])
            .filter(child => !child.isPlaceholder);
          
          // 创建新子节点映射
          const newChildrenMap = new Map(newChildren.map(child => [child.id, child]));
          
          // 合并现有节点和新节点
          const mergedChildren = [
            ...existingChildren.filter(child => !newChildrenMap.has(child.id)),
            ...newChildren
          ];
          
          // 对子节点进行排序
          const sortedChildren = sortProjectsByType(mergedChildren);
          
          return {
            ...node,
            children: sortedChildren,
            hasChildrenLoaded: true
          };
        })
      );
      
      // 确保目标节点保持展开状态
      setExpandedNodes(prev => {
        if (!prev.includes(nodeId)) {
          return [...prev, nodeId];
        }
        return prev;
      });
      
      // 重置加载状态
      setLoadingState(null);
    }
  }, [treeItems, isLoading, loadingState, sortProjectsByType, updateTreeNode]);

  // 处理目录标签点击
  const handleLabelClick = useCallback((path: string, isDirectory: boolean) => {
    if (!isDirectory) return;
    
    // 构建完整路径并移除所有连续斜杠
    const fullPath = `/${scope}/code/tree${path}`;
    const cleanPath = fullPath.replace(/\/+/g, '/');
    
    // 更新 URL
    router.push(cleanPath);

    // 查找目标节点
    const targetNode = findNode(treeData, path);

    if (!targetNode) return;

    // 确保节点被展开
    setExpandedNodes(prev => {
      if (prev.includes(path)) {
        return prev;
      }
      return [...prev, path];
    });

    // 总是重新加载节点数据
    loadedNodes.current.delete(path); // 移除标记以重新加载
    
    // 设置加载状态
    setLoadingState({
      path: targetNode.path,
      nodeId: targetNode.id,
      isRefreshing: true
    });

    // 设置节点加载状态
    setTreeData(prev => 
      updateTreeNode(prev, targetNode.id, node => ({
        ...node,
        hasChildrenLoaded: false
      }))
    );
  }, [router, scope, findNode, treeData, updateTreeNode]);

  // 选择节点
  const handleNodeSelect = useCallback(
    (_event: React.SyntheticEvent<Element, Event> | null, nodeId: string | null) => {
      if (!nodeId) return;
      
      setSelectedNode(nodeId);
      const node = findNode(treeData, nodeId);
      
      if (!node) return;
      
      // 只有当节点是文件时才跳转
      if (node.content_type === 'file' && flag === 'contents') {
        // 直接使用节点的完整路径构建跳转URL
        const filePath = node.path.startsWith('/') ? node.path : `/${node.path}`;

        router.push(`/${scope}/code/blob${filePath}`);
      }
    },
    [findNode, treeData, scope, flag, router],
  );

  // 将变化的basePath传回父组件，请求commit信息
  useEffect(() => {

    if (basePath) {
      // 更改commit接口的path
      onCommitInfoChange?.(basePath);
    }
  }, [basePath, onCommitInfoChange]);

  return (
    <>
      {isInitialLoading ? (
        <Box sx={{ display: 'flex', justifyContent: 'center', padding: '16px' }}>
          <Skeleton variant="rounded" width="100%" height={24} />
        </Box>
      ) 
      : treeData?.length === 0 ? (
        <Box sx={{ display: 'flex', paddingLeft: '16px' }}>
          <Skeleton width="200px" height="30px" />
        </Box>
      ) 
      : (
        <RichTreeView
          items={treeData}
          expandedItems={expandedNodes}
          selectedItems={selectedNode}
          onExpandedItemsChange={handleNodeToggle}
          onSelectedItemsChange={handleNodeSelect}
          sx={{ height: 'fit-content', flexGrow: 1, width: 400, overflowY: 'auto' }}
          slots={{
            item: (itemProps) => (
              <CustomTreeItem 
                {...itemProps}
                onLabelClick={handleLabelClick}
              />
            )
          }}
        />
      )}
    </>
  );
};

export default RepoTree;