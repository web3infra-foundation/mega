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
* @param data: Array of tree nodes
* @param nodeId: ID of the node to search for
* @returns: Found node or null
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

/**
* Get all parent node IDs for a specified path
* @param path: A path string, such as '/a/b/c'
* @returns: An array of parent node IDs, such as ['/a', '/a/b', '/a/b/c']
*/
export const getAllParentIds = (path: string): string[] => {
  if (!path || path === '/') return [];
  
  // Split the path into its parts
  const parts = path.split('/').filter(part => part !== '');
  
  // Generate all parent node IDs (including itself)
  const parentIds: string[] = [];
  let currentPath = '';
  
  for (const part of parts) {
    currentPath = currentPath ? `${currentPath}/${part}` : `/${part}`;
    parentIds.push(currentPath);
  }
  
  return parentIds;
};

/**
* Generates an array of node paths to be expanded based on the path.
* @param path: A path string, such as '/a/b/c'
* @returns: An array of expanded paths, such as ['/', '/a', '/a/b', '/a/b/c']
*/
export const generateExpandedPaths = (path: string): string[] => {
  if (path === '/') return ['/'];
  
  const segments = path.split('/').filter(Boolean);
  const expandedPaths: string[] = ['/'];
  
  let currentPath = '';

  segments.forEach(segment => {
    currentPath += `/${segment}`;
    expandedPaths.push(currentPath);
  });

  return expandedPaths;
};

export const convertToTreeData = (responseData: any): MuiTreeNode[] => {
  if (!responseData?.data) {
    return [];
  }
  const data = responseData.data;
  
  // Create a mapping of paths to nodes
  const pathToNodeMap = new Map<string, MuiTreeNode>();
  
  // 1. Process all paths in file_tree, sort by path length and ensure that parent nodes are created first
  if (data.file_tree) {
    // Fixed sorting logic: ensure parent paths are processed before child paths
    const sortedPaths = Object.keys(data.file_tree).sort((a, b) => {
      const aDepth = a === '/' ? 0 : a.split('/').length;
      const bDepth = b === '/' ? 0 : b.split('/').length;

      return aDepth - bDepth;
    });
    
    sortedPaths.forEach(path => {
      const pathData = data.file_tree[path];

      if (!pathData) return;
      
      // Process all subitems under the current path
      pathData.tree_items.forEach((item: any) => {
        const itemPath = item.path;
        const parentPath = itemPath.substring(0, itemPath.lastIndexOf('/')) || '/';
        
        // Create the current node
        const currentNode: MuiTreeNode = {
          id: itemPath,
          label: item.name,
          path: itemPath,
          content_type: item.content_type,
          children: item.content_type === 'directory' ? [] : undefined
        };
        
        pathToNodeMap.set(itemPath, currentNode);
        
        // Add the current node to the children of the parent node
        if (parentPath !== '/') {
          const parentNode = pathToNodeMap.get(parentPath);

          if (parentNode && parentNode.children) {
            parentNode.children.push(currentNode);
          }
        }
      });
    });
  }
  
  // 2. Process tree_items, updating existing nodes or adding new nodes
  if (data.tree_items) {
    data.tree_items.forEach((item: any) => {
      const itemPath = item.path;
      const parentPath = itemPath.substring(0, itemPath.lastIndexOf('/')) || '/';
      
      let currentNode = pathToNodeMap.get(itemPath);
      
      if (!currentNode) {
        // Create a new node
        currentNode = {
          id: itemPath,
          label: item.name,
          path: itemPath,
          content_type: item.content_type,
          children: item.content_type === 'directory' ? [] : undefined
        };
        pathToNodeMap.set(itemPath, currentNode);
      } else {
        // Updating an existing node
        currentNode.label = item.name;
        currentNode.content_type = item.content_type;
        if (item.content_type === 'directory' && !currentNode.children) {
          currentNode.children = [];
        }
      }
      
      // Adding a node to a parent node
      if (parentPath !== '/') {
        const parentNode = pathToNodeMap.get(parentPath);

        if (parentNode && parentNode.children) {
          // Check if it already exists
          const existingIndex = parentNode.children.findIndex(child => child.path === itemPath);

          if (existingIndex >= 0) {
            parentNode.children[existingIndex] = currentNode;
          } else {
            parentNode.children.push(currentNode);
          }
        }
      }
    });
  }
  
  // 3. Build the list of children nodes of the root directory - fix the logic
  const result: MuiTreeNode[] = [];
  
  // Collect all root-level nodes (nodes whose parent path is '/')
  pathToNodeMap.forEach((node, path) => {
    const parentPath = path.substring(0, path.lastIndexOf('/')) || '/';

    if (parentPath === '/') {
      result.push(node);
    }
  });
  
  // 4. Add placeholders for empty directories (but only for directories that actually have no children)
  const addPlaceholdersToEmptyDirectories = (nodes: MuiTreeNode[]) => {
    nodes.forEach(node => {
      if (node.content_type === 'directory') {
        // Checks if there are actual child nodes (excluding placeholders)
        const hasRealChildren = node.children && node.children.some(child => !child.isPlaceholder);
        
        if (!hasRealChildren) {
          // A truly empty directory, add a placeholder
          node.children = [{ 
            id: `${node.path}-placeholder`, 
            label: 'placeholder', 
            path: `${node.path}-placeholder`, 
            content_type: 'file', 
            children: undefined, 
            isPlaceholder: true 
          }];
        } else {
          // Directories with subitems, processed recursively
          if (node.children) {
            addPlaceholdersToEmptyDirectories(node.children);
          }
        }
      }
    });
  };
  
  addPlaceholdersToEmptyDirectories(result);
  
  // 5. Sort the children of each node
  const sortNodeChildren = (nodes: MuiTreeNode[]) => {
    nodes.forEach(node => {
      if (node.children && node.children.length > 0) {
        // Filter out placeholders and then sort
        const nonPlaceholderChildren = node.children.filter(child => !child.isPlaceholder);

        if (nonPlaceholderChildren.length > 0) {
          node.children = sortProjectsByType(nonPlaceholderChildren);
          sortNodeChildren(node.children);
        }
      }
    });
  };
  
  sortNodeChildren(result);
  
  // 6. Sort the root nodes and return them
  return sortProjectsByType(result);
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
* @param treeData: The complete tree data.
* @param nodeId: The ID of the parent node to be found.
* @returns: An array of the IDs of all descendant nodes under the node.
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

