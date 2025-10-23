'use client'

import React, { useEffect } from 'react'
import { useAtom } from 'jotai'
// import { ChevronRightIcon, ChevronLeftIcon } from '@heroicons/react/20/solid'

import { treeCollapsedAtom } from './codeTreeAtom'
import RepoTree from './RepoTree'

interface CollapsibleTreeViewProps {
  onCommitInfoChange?: (path: string) => void
}

const CollapsibleTreeView = ({ onCommitInfoChange }: CollapsibleTreeViewProps) => {
  const [isCollapsed, setIsCollapsed] = useAtom(treeCollapsedAtom)

  useEffect(() => {
    const handleKeyPress = (e: KeyboardEvent) => {
      if (e.key === ']') {
        setIsCollapsed((prev) => !prev)
      }
    }

    window.addEventListener('keydown', handleKeyPress)

    return () => {
      window.removeEventListener('keydown', handleKeyPress)
    }
  }, [setIsCollapsed])

  const treeStyle: React.CSSProperties = {
    borderRadius: 8,
    width: isCollapsed ? '0' : '20%',
    minWidth: isCollapsed ? '0' : '300px',
    flexShrink: 0,
    background: '#fff',
    height: 'calc(100vh - 96px)',
    overflow: isCollapsed ? 'hidden' : 'auto',
    paddingRight: isCollapsed ? '0' : '8px',
    transition: 'all 0.3s ease-in-out',
    position: 'relative',
    opacity: isCollapsed ? 0 : 1
  }

//   const toggleButtonStyle: React.CSSProperties = {
//     position: 'absolute',
//     top: '10px',
//     right: '8px',
//     zIndex: 10,
//     cursor: 'pointer',
//     padding: '4px',
//     borderRadius: '4px',
//     backgroundColor: '#f3f4f6',
//     display: 'flex',
//     alignItems: 'center',
//     justifyContent: 'center',
//     transition: 'background-color 0.2s'
//   }

  return (
    <div style={treeStyle}>
      {/* <button
        style={toggleButtonStyle}
        onClick={() => setIsCollapsed(!isCollapsed)}
        onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = '#e5e7eb')}
        onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = '#f3f4f6')}
        title={isCollapsed ? '展开目录树 (按 ] 键)' : '收起目录树 (按 ] 键)'}
      >
        {isCollapsed ? (
          <ChevronRightIcon className='size-4 text-gray-600' />
        ) : (
          <ChevronLeftIcon className='size-4 text-gray-600' />
        )}
      </button> */}
      <div className={`${isCollapsed ? 'hidden' : 'block'}`}>
        <RepoTree onCommitInfoChange={onCommitInfoChange} />
      </div>
    </div>
  )
}

export default CollapsibleTreeView

