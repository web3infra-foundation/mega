import React, { useEffect, useRef } from 'react'

interface VersionSelectorDropdownProps {
  isOpen: boolean
  onClose: () => void
  onVersionSelect: (version: string) => void
  currentVersion: string
  versions: string[]
}

export const VersionSelectorDropdown: React.FC<VersionSelectorDropdownProps> = ({
  isOpen,
  onClose,
  onVersionSelect,
  currentVersion,
  versions
}) => {
  const dropdownRef = useRef<HTMLDivElement>(null)

  const handleVersionSelect = (version: string) => {
    onVersionSelect(version)
    onClose()
  }

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        onClose()
      }
    }

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside)
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [isOpen, onClose])

  if (!isOpen) return null

  return (
    <div
      ref={dropdownRef}
      className='absolute left-0 top-full z-50 mt-1 w-full rounded-md border border-gray-300 bg-white shadow-lg'
      style={{ width: '154px' }}
    >
      <div className='p-3'>
        <div className='max-h-48 overflow-y-auto'>
          <div className='mb-0'>
            <div className='mb-2 pl-0 text-xs font-medium uppercase tracking-wide text-gray-500'>Default</div>
            <div className='space-y-1'>
              <button
                onClick={() => handleVersionSelect(currentVersion)}
                className='w-full rounded px-0 py-1 text-left hover:bg-gray-100'
              >
                <span className='text-sm text-gray-900'>{currentVersion}</span>
              </button>
            </div>
          </div>

          <div>
            <div
              style={{
                display: 'flex',
                padding: 'var(--Spacing-2, 8px) var(--Spacing-3, 12px)',
                alignItems: 'center',
                alignSelf: 'stretch',
                background: '#ffffff00',
                marginTop: '1px',
                marginBottom: '6px'
              }}
            >
              <div
                className='w-full bg-gray-200'
                style={{ marginLeft: '-12px', marginRight: '-2px', height: '1.5px' }}
              ></div>
            </div>
            <div className='mb-2 pl-0 text-xs font-medium uppercase tracking-wide text-gray-500'>ALL</div>
            <div className='space-y-1'>
              {versions.map((version: string) => (
                <button
                  key={version}
                  onClick={() => handleVersionSelect(version)}
                  className='w-full rounded px-0 py-1 text-left hover:bg-gray-100'
                >
                  <span className='text-sm text-gray-900'>{version}</span>
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className='mt-3 border-t pt-3'>
          <button
            onClick={() => {
              //  console.log('View all versions');
            }}
            style={{
              flex: '1 0 0',
              color: '#3a5bc7',
              fontFamily: '"SF Pro"',
              fontSize: '14px',
              fontStyle: 'normal',
              fontWeight: 400,
              lineHeight: '20px',
              letterSpacing: 'var(--Typography-Letter-spacing-2, 0)'
            }}
          >
            View all versions
          </button>
        </div>
      </div>
    </div>
  )
}
