import * as React from 'react';
import { useCallback, useEffect, useState } from 'react';
import { Box, Skeleton } from '@mui/material';
import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import { usePathname } from 'next/navigation';
import { useRouter } from 'next/router';
import { useGetTree } from '@/hooks/useGetTree';
import { legacyApiClient } from '@/utils/queryClient';
import { convertToTreeData, generateExpandedPaths, mergeTreeNodes, findNode } from './TreeUtils';
import { CustomTreeItem } from './CustomTreeItem';
import toast from 'react-hot-toast';
import { useAtom } from 'jotai';
import { expandedNodesAtom, treeAllDataAtom } from './codeTreeAtom';

const RepoTree = ({ onCommitInfoChange }: { onCommitInfoChange?:Function }) => {
  const router = useRouter();
  const scope = router.query.org as string;
  const pathname = usePathname();
  const basePath = pathname?.replace(
    new RegExp(`\\/${scope}\\/code\\/(tree|blob)`), 
    ''
  ) || '/'; 

  const [treeAllData, setTreeAllData] = useAtom(treeAllDataAtom)
  const [expandedNodes, setExpandedNodes] = useAtom(expandedNodesAtom)
  const [loadingDirectories, setLoadingDirectories] = useState<Set<string>>(new Set());
  
  const { data: treeItems } = useGetTree({ path: basePath });

  // Set the expanded state on initialization
  useEffect(() => {
    const pathsToExpand = generateExpandedPaths(basePath);
    const existingNode = findNode(treeAllData, basePath);
    const hasRealData = existingNode?.children && !existingNode?.children[0].isPlaceholder
    
    if (!loadingDirectories.has(basePath) && !hasRealData && existingNode?.content_type === 'directory') {
      setLoadingDirectories(prev => new Set(prev).add(basePath));
    }
    setExpandedNodes(Array.from(new Set([...expandedNodes, ...pathsToExpand])));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [basePath]);

  useEffect(() => {
    if (treeItems) {
      const newPathTreeData = convertToTreeData(treeItems)
      const newTreeAllData = mergeTreeNodes(treeAllData, newPathTreeData)

      setTreeAllData(newTreeAllData)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [treeItems]);

  const handleNodeToggle = useCallback((_event: React.SyntheticEvent | null, nodeIds: string[]) => {
    const newlyExpandedIds = nodeIds.filter(id => !expandedNodes.includes(id));
    
    newlyExpandedIds.forEach(nodeId => {
      const existingNode = findNode(treeAllData, nodeId);
      const hasRealData = existingNode?.children && !existingNode?.children[0].isPlaceholder
      
      if (!loadingDirectories.has(nodeId) && !hasRealData) {
        setLoadingDirectories(prev => new Set(prev).add(nodeId));
      }
    });

    setExpandedNodes(nodeIds);
  }, [expandedNodes, loadingDirectories, treeAllData, setLoadingDirectories, setExpandedNodes]);

  useEffect(() => {
    loadingDirectories.forEach(path => {
      legacyApiClient.v1.getApiTree().request({ path })
        .then((response: any) => {
          if (response) {   
            const newDirectoryData = convertToTreeData(response)
            const newTreeAllData = mergeTreeNodes(treeAllData, newDirectoryData)

            setTreeAllData(newTreeAllData)
          }
        })
        .catch((_error: any) => {
          toast.error('Loading failed.');
        })
        .finally(() => {
          setLoadingDirectories(prev => {
            const newSet = new Set(prev);

            newSet.delete(path);
            return newSet;
          });
        });
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loadingDirectories]);

  const handleLabelClick = useCallback((path: string, isDirectory: boolean) => {
    if (isDirectory) {
      const fullPath = `/${scope}/code/tree${path}`;
      const cleanPath = fullPath.replace(/\/+/g, '/');
      
      router.push(cleanPath);
    } else {      
      const filePath = path.startsWith('/') ? path : `/${path}`;

      router.push(`/${scope}/code/blob${filePath}`);
    }
  }, [router, scope]);

  useEffect(() => {
    if (basePath) {
      onCommitInfoChange?.(basePath);
    }
  }, [basePath, onCommitInfoChange]);

  return (
    <>
      {treeAllData?.length === 0 ? (
        <Box sx={{ display: 'flex', paddingLeft: '16px' }}>
          <Skeleton width="200px" height="30px" />
        </Box>
      ) 
      : (
        <RichTreeView
          items={treeAllData}
          expandedItems={expandedNodes}
          onExpandedItemsChange={handleNodeToggle}
          sx={{ height: 'fit-content', flexGrow: 1, width: 400, overflow: 'auto' }}
          slots={{
            item: (itemProps) => (
              <CustomTreeItem 
                {...itemProps}
                onLabelClick={handleLabelClick}
                loadingDirectories={loadingDirectories}
              />
            )
          }}
        />
      )}
    </>
  );
};

export default RepoTree;