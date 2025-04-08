import { useState } from 'react';

const TreeNode = ({ node, handleNodeClick, loadChildren }) => {
    const [isExpanded, setIsExpanded] = useState(false);
    const [childrenLoaded, setChildrenLoaded] = useState(false);

    const handleToggle = () => {
        if (!childrenLoaded && node.content_type === 'directory') {
            loadChildren(node.id); // load subdirectory
            setChildrenLoaded(true);
        }
        setIsExpanded(!isExpanded);
        handleNodeClick(node);
    };

    return (
        <div>
            <div onClick={handleToggle}>
                {node.content_type === 'directory' && (
                    <span>
                        {isExpanded ? '▼' : '►'}
                    </span>
                )}
                {node.name}
            </div>
            {isExpanded && node.children && node.children.map(childNode => (
                <div key={childNode.id} style={{ marginLeft: '50px' }}>
                    <TreeNode node={childNode} handleNodeClick={handleNodeClick} loadChildren={loadChildren} />
                </div>
            ))}
        </div>
    );
};

const Tree = ({ data, handleNodeClick, loadChildren }) => {
    return (
        <div>
            {data.map(node => (
                <div key={node.id}>
                    <TreeNode node={node} handleNodeClick={handleNodeClick} loadChildren={loadChildren} />
                </div>
            ))}
        </div>
    );
};

export default Tree;
