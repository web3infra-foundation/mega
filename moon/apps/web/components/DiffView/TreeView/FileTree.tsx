import * as React from 'react';
import { useCallback, useEffect, useMemo } from 'react';
import { Box, Skeleton } from '@mui/material';
import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import { convertToTreeData, getDescendantIds } from './TreeUtils';
import { CustomTreeItem } from './CustomTreeItem';
import { useAtom } from 'jotai';
import { expandedNodesAtom, treeAllDataAtom } from './codeTreeAtom';
import { useTreeViewApiRef } from "@mui/x-tree-view";
import { useGetMrFileTree } from "@/hooks/useGetMrFileTree";
import { usePathname } from "next/navigation";

const FileTree = ({ link, onFileClick }: { link: string; onFileClick?: (filePath: string) => void }) => {
  const apiRef = useTreeViewApiRef();

  const [treeAllData, setTreeAllData] = useAtom(treeAllDataAtom)
  const [expandedNodes, setExpandedNodes] = useAtom(expandedNodesAtom)

  const pathname = usePathname()!!;
  const orgPath = useMemo(() => pathname.split('/').at(1), [pathname]);
  const { data: treeResponse, isLoading } = useGetMrFileTree(link, orgPath)

  // Process the tree data when API returns
  useEffect(() => {
    if (treeResponse?.data) {
      const convertedData = convertToTreeData(treeResponse.data)

      setTreeAllData(convertedData)
    }
  }, [treeResponse, setTreeAllData]);

  const handleNodeToggle = useCallback((_event: React.SyntheticEvent | null, nodeIds: string[]) => {
    const collapsedNodes = expandedNodes.filter(id => !nodeIds.includes(id));

    let newExpandedIds = [...nodeIds];

    if (collapsedNodes.length > 0) {
      collapsedNodes.forEach(collapsedId => {
        const descendantIds = getDescendantIds(treeAllData, collapsedId);

        newExpandedIds = newExpandedIds.filter(id => !descendantIds.includes(id));
      });
    }

    setExpandedNodes(newExpandedIds);
  }, [expandedNodes, treeAllData, setExpandedNodes]);

  const handleItemClick = (_e: React.SyntheticEvent | null, itemId: string) => {
    const item = apiRef.current!.getItem(itemId)

    if (item.content_type) {
      // Call the file click handler if this is a file (not directory)
      if (item.content_type === 'file' && onFileClick) {
        onFileClick(item.path || itemId)
      }
    }
  }

  const handleFocusItem = (_e: React.SyntheticEvent | null, itemId: string) => {
    const item = apiRef.current!.getItem(itemId)

    if (item.content_type) {
      apiRef.current?.setItemSelection({
        itemId,
        keepExistingSelection: false
      })

      const newExpandedIds = [...expandedNodes, itemId]

      setExpandedNodes(newExpandedIds)
    }
  }

  return (
    <>
      {isLoading || treeAllData?.length === 0 ? (
          <Box sx={{ display: 'flex', paddingLeft: '16px' }}>
            <Skeleton width="200px" height="30px" />
          </Box>
        )
        : (
          <RichTreeView
            apiRef={apiRef}
            items={treeAllData}
            onItemFocus={handleFocusItem}
            onItemClick={handleItemClick}
            expandedItems={expandedNodes}
            onExpandedItemsChange={handleNodeToggle}
            sx={{ flexGrow: 1, width: '100%', overflow: 'auto' }}
            slots={{
              item: (itemProps) => (
                <CustomTreeItem
                  {...itemProps}
                />
              )
            }}
          />
        )}
    </>
  );
};

export default FileTree;