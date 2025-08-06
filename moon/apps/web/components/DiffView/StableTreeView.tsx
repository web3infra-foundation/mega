import { memo, useRef } from 'react'
import { FileIcon } from '@primer/octicons-react'
import { TreeView } from '@primer/react'

import { MuiTreeNode } from '@gitmono/types/generated'

const TreeViewItem = TreeView.Item
const TreeViewSub = TreeView.SubTree

const StableTreeView = ({
  treeData,
  handleClick
}: {
  treeData: MuiTreeNode[]
  handleClick?: (file: string) => void
}) => {
  const currentId = useRef('')

  return (
    <TreeView aria-label='Files changed'>
      <TreeLoop nodes={treeData} onSelect={(file) => handleClick?.(file)} currentId={currentId} />
    </TreeView>
  )
}

const TreeLoop = ({
  nodes,
  basePath,
  onSelect,
  currentId
}: {
  nodes: MuiTreeNode[]
  basePath?: string
  onSelect?: (path: string) => void
  currentId: React.MutableRefObject<string>
}) => {
  return (
    <>
      {nodes.map((node) => {
        const currentPath = basePath ? `${basePath}/${node.label}` : node.label

        return (
          <TreeViewItem
            id={node.id}
            key={node.id}
            defaultExpanded
            onSelect={() => {
              currentId.current = node.id
              onSelect?.(currentPath)
            }}
            current={currentId.current === node.id && !node.children}
          >
            <TreeView.LeadingVisual>
              {node.children && node.children.length > 0 ? <TreeView.DirectoryIcon /> : <FileIcon />}
            </TreeView.LeadingVisual>
            {node.label}

            {node.children && node.children.length > 0 && (
              <TreeViewSub>
                <TreeLoop nodes={node.children} basePath={currentPath} onSelect={onSelect} currentId={currentId} />
              </TreeViewSub>
            )}
          </TreeViewItem>
        )
      })}
    </>
  )
}

export default memo(StableTreeView)
