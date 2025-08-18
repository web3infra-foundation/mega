import * as React from 'react';
import { TreeItemLabel } from '@mui/x-tree-view/TreeItem';
import { Box } from '@mui/material';

interface CustomLabelProps {
  children: React.ReactNode;
  icon?: React.ElementType;
  expandable?: boolean;
  onClick?: (event: React.MouseEvent) => void;
}

// Custom label component used to render each node in the tree structure
export function CustomLabel({ icon: Icon, children, onClick, ...other }: CustomLabelProps) {
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
