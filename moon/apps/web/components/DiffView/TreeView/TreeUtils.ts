import ArticleIcon from '@mui/icons-material/Article';
import FolderRounded from '@mui/icons-material/FolderRounded';
import FolderOpenIcon from '@mui/icons-material/FolderOpen';

export interface MuiTreeNode {
  id: string;
  label: string;
  path: string;
  content_type?: 'file' | 'directory';
  children?: MuiTreeNode[];
  isPlaceholder?: boolean;
}

// Custom icon function, returns different icon components according to file type
export const getIconFromFileType = (fileType: 'file' | 'directory' | undefined, isExpanded: boolean) => {
    switch (fileType) {
      case 'file':
        return ArticleIcon;
      case 'directory':
        return isExpanded ? FolderOpenIcon : FolderRounded;
      default:
        return ArticleIcon;
    }
  };

// Sort tree nodes: directories first, files second, and the same type sorted in alphabetical order by name
export const sortProjectsByType = (projects: MuiTreeNode[]): MuiTreeNode[] => {
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
};

/**
 * Recursively search for a node with a specified ID in the tree.
 * @returns: Found node or null
 * @param data
 * @param nodeId
 */
export const findNode = (data: MuiTreeNode[], nodeId: string): MuiTreeNode | null => {
  for (const node of data) {
    if (node.id === nodeId) return node;
    if (node.children) {
      const found = findNode(node.children, nodeId);

      if (found) return found;
    }
  }
  return null;
};

export const convertToTreeData = (responseData: any): MuiTreeNode[] => {
  if (!responseData) {
    return [];
  }

  // Handle the new API format where responseData is the tree structure directly
  const convertNode = (node: any, parentPath: string = ''): MuiTreeNode => {
    // Build the full path by combining parent path with current node label
    const fullPath = parentPath ? `${parentPath}/${node.label}` : node.label

    return {
      id: node.id,
      label: node.label,
      path: fullPath, // Use the full file path instead of just node.id
      content_type: node.children? 'directory' : 'file',
      children: node.children? node.children.map((child: any) => convertNode(child, fullPath)) : undefined
    };
  };

  // Convert the tree structure
  if (Array.isArray(responseData)) {
    const result = responseData.map((node: any) => convertNode(node, ''));

    return sortProjectsByType(result);
  }

  return [];
};

function mergeNode(node1: MuiTreeNode, node2: MuiTreeNode): MuiTreeNode {
  if (node1.isPlaceholder && node2.isPlaceholder) {
    return { ...node1 };
  }
  if (node1.isPlaceholder && !node2.isPlaceholder) {
    return { ...node2 };
  }
  if (!node1.isPlaceholder && node2.isPlaceholder) {
    return { ...node1 };
  }

  const merged: MuiTreeNode = { ...node1 };

  if (node1.content_type === 'directory' || node2.content_type === 'directory') {
    const children1 = node1.children || [];
    const children2 = node2.children || [];
    
    merged.children = mergeTreeNodes(children1, children2);
  }
  return merged;
}

export function mergeTreeNodes(nodes1: MuiTreeNode[], nodes2: MuiTreeNode[]): MuiTreeNode[] {
  const map = new Map<string, MuiTreeNode>();

  for (const node of nodes1) {
    map.set(node.path, { ...node });
  }

  for (const node of nodes2) {
    if (map.has(node.path)) {
      const existing = map.get(node.path)!;

      map.set(node.path, mergeNode(existing, node));
    } else {
      map.set(node.path, { ...node });
    }
  }

  const result = Array.from(map.values()).map(n => {
    if (n.children) {
      n.children = sortProjectsByType(mergeTreeNodes(n.children, []));
    }
    return n;
  });

  return sortProjectsByType(result);
}

/**
 * Recursively retrieves the IDs of all descendant nodes of a node.
 * @returns: An array of the IDs of all descendant nodes under the node.
 * @param treeData
 * @param nodeId
 */
export function getDescendantIds(treeData: MuiTreeNode[], nodeId: string): string[] {
  const node = findNode(treeData, nodeId);

  if (!node || !node.children) {
    return [];
  }

  let ids: string[] = [];
  
  node.children.forEach(child => {
    ids.push(child.id);
    ids = ids.concat(getDescendantIds(treeData, child.id));
  });

  return ids;
}

