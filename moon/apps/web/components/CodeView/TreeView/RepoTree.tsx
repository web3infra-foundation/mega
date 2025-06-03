import * as React from 'react';
import { useState, useEffect, useCallback } from 'react';
import { usePathname } from 'next/navigation';
import { useRouter } from 'next/router';
import FolderRounded from '@mui/icons-material/FolderRounded';
import ArticleIcon from '@mui/icons-material/Article';
import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import { TreeItemCheckbox, TreeItemContent, TreeItemGroupTransition, TreeItemIconContainer, TreeItemLabel, TreeItemRoot } from '@mui/x-tree-view/TreeItem';
import { TreeItemDragAndDropOverlay, TreeItemIcon, TreeItemProvider, useTreeItem, useTreeItemModel, UseTreeItemParameters } from '@mui/x-tree-view';
import { Box, IconButton } from '@mui/material';

interface MuiTreeNode {
  id: string;
  label: string;
  path: string;
  content_type:string;
  // isLeaf: boolean;
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
  
  function CustomLabel({
    icon: Icon,
    children,
    ...other
  }: CustomLabelProps) {
    return (
      <TreeItemLabel
        {...other}
        sx={{
          display: 'flex',
          alignItems: 'center',
        }}
      >
        {Icon && (
          <Box
            component={Icon}
            className="labelIcon"
            color="inherit"
            sx={{ mr: 1, fontSize: '1.2rem' }}
          />
        )}

        <TreeItemLabel>{children}</TreeItemLabel>
      </TreeItemLabel>
    );
  }
    
  const getIconFromFileType = (fileType: FileType) => {
    switch (fileType) {
      case 'file':
        return ArticleIcon;
      case 'directory':
        return FolderRounded;
      default:
        return ArticleIcon;
    }
  };
  
  interface CustomTreeItemProps
    extends Omit<UseTreeItemParameters, 'rootRef'>,
      Omit<React.HTMLAttributes<HTMLLIElement>, 'onFocus'> {}

  const CustomTreeItem = React.forwardRef(function CustomTreeItem(
    props: CustomTreeItemProps,
    ref: React.Ref<HTMLLIElement>,
  ) {
    const { id, itemId, label, disabled, children, ...other } = props;
    const {
      getContextProviderProps,
      getRootProps,
      getContentProps,
      getIconContainerProps,
      getCheckboxProps,
      getLabelProps,
      getGroupTransitionProps,
      getDragAndDropOverlayProps,
      status,
    } = useTreeItem({ id, itemId, children, label, disabled, rootRef: ref });
  
    const item = useTreeItemModel<ExtendedTreeItemProps>(itemId)!;

    let icon;

    if (status.expandable) {
      icon = FolderRounded;
    } else if (item.content_type) {
      icon = getIconFromFileType(item.content_type);
    }

    const handleClick = async(event: React.MouseEvent) => {
      // eslint-disable-next-line no-console
      console.log(event,"event===event====")
      // interactions.handleExpansion(event);
  
    };

    return (
      <TreeItemProvider {...getContextProviderProps()}>
        <TreeItemRoot {...getRootProps(other)}>
          <TreeItemContent {...getContentProps()}>
            
            <TreeItemIconContainer {...getIconContainerProps()}>
              <TreeItemIcon status={status} />
            </TreeItemIconContainer>
  
              {/* icon */}
            <React.Fragment>
              <IconButton
                onClick={handleClick}
                aria-label="collapse item"
                size="small"
              >
              </IconButton>
            </React.Fragment>
            
            <TreeItemCheckbox {...getCheckboxProps()} />
  
            {/* label */}
            <CustomLabel
              {...getLabelProps({
                icon,
                expandable: status.expandable && status.expanded,
              })}
            />
            <TreeItemDragAndDropOverlay {...getDragAndDropOverlayProps()} />
          </TreeItemContent>
          {children && <TreeItemGroupTransition {...getGroupTransitionProps()} />}
        </TreeItemRoot>
      </TreeItemProvider>
    );
  }
);

const RepoTree = ({ directory }:any) => {
  const router = useRouter();
  const pathname = usePathname();
  const [treeData, setTreeData] = useState<MuiTreeNode[]>([]);
  const [expandedNodes, setExpandedNodes] = useState<string[]>([]);
  const [selectedNode, setSelectedNode] = useState<string | null>(null);

  const convertToTreeData = useCallback((directory:any) => {
    return sortProjectsByType(directory)?.map(item => ({
      id: item.date + item.name,
      label: item.name,
      path: item.path,
      content_type:item.content_type,
      children: item.content_type === 'directory' ? [] : undefined
    }));
  }, []);

  useEffect(() => {
    setTreeData(convertToTreeData(directory));
  }, [directory, convertToTreeData]);


  const sortProjectsByType = (projects:any[]) => {
    return projects?.sort((a, b) => {
      if (a.content_type === 'directory' && b.content_type === 'file') {
        return -1;
      } else if (a.content_type === 'file' && b.content_type === 'directory') {
        return 1;
      } else {
        return 0;
      }
    });
  };

  const appendTreeData = (treeData: any, subItems: any, nodeId: string) => {
    return treeData.map((item:any) => {
      if (item.id === nodeId) {
        return {
          ...item,
          children: subItems
        };
      } else if (item.children) {
        return {
          ...item,
          children: appendTreeData(item.children, subItems, nodeId)
        };
      }
      return item;
    });
  };

  const handleNodeToggle = (_event: React.SyntheticEvent<Element, Event> | null, nodeIds: string[]) => {
    const newlyExpandedNodeId = nodeIds.find(id => !expandedNodes.includes(id));
    

    // eslint-disable-next-line no-console
    console.log("11111====")
    if (newlyExpandedNodeId) {
      const loadChildren = async () => {
        try {
          const node = findNode(treeData, newlyExpandedNodeId);

          // eslint-disable-next-line no-console
          console.log(node, 'node===node====')

          if (!node) return;

          let responseData;
          const reqPath = pathname?.replace('/tree', '') + '/' + node.label;
          
          if (node.path && node.path !== '' && node.path !== undefined) {
            responseData = await fetch(`/api/tree?path=${node.path}`)
              .then(response => response.json());
          } else {
            responseData = await fetch(`/api/tree?path=${reqPath}`)
              .then(response => response.json());
          }

          const subTreeData = convertToTreeData(responseData.data.data);
          const newTreeData = appendTreeData(treeData, subTreeData, newlyExpandedNodeId);

          setTreeData(newTreeData);
        } catch (error) {
          // eslint-disable-next-line no-console
          console.error('Error fetching tree data:', error);
        }
      };
      
      loadChildren();
    }
    
    setExpandedNodes(nodeIds);
  };

  const handleNodeSelect = (_event: React.SyntheticEvent<Element, Event> | null, nodeId: string | null) => {
    if (!nodeId) return;

    setSelectedNode(nodeId);
    const node = findNode(treeData, nodeId);

    // eslint-disable-next-line no-console
    console.log("node====")

    if (!node) return;

    const real_path = pathname?.replace('/tree', '');
    const modified_path = real_path?.replace('/code/', '/code/blob/');

    if (node?.content_type === 'directory') {
      router.push(`${pathname}/${node.label}`);
    } else {
      router.push(`/${modified_path}/${node.label}`);
    }
  };

  const findNode = (data: MuiTreeNode[], nodeId: string): MuiTreeNode | null => {
    for (const node of data) {
      if (node.id === nodeId) return node;
      if (node.children) {
        const found = findNode(node.children, nodeId);

        if (found) return found;
      }
    }
    return null;
  };

  return (
    <RichTreeView
    items={treeData}
    expandedItems={expandedNodes}
    selectedItems={selectedNode}
    onExpandedItemsChange={handleNodeToggle}
    onSelectedItemsChange={handleNodeSelect}
    sx={{ height: 'fit-content', flexGrow: 1, maxWidth: 400, overflowY: 'auto' }}
    slots={{ item: CustomTreeItem }}
  />
  )
};

export default RepoTree;