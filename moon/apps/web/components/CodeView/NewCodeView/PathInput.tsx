'use client'

import React, { KeyboardEvent, useEffect, useState } from 'react'
import { useParams } from 'next/navigation'

interface PathInputProps {
  pathState: [string, React.Dispatch<React.SetStateAction<string>>]
  nameState: [string, React.Dispatch<React.SetStateAction<string>>]
}

export default function PathInput({ pathState, nameState }: PathInputProps) {
  const [, setPath] = pathState
  const [, setName] = nameState
  const params = useParams()

  const [basePath, setBasePath] = useState<string[]>([])

  const [userSegments, setUserSegments] = useState<string[]>([])
  const [current, setCurrent] = useState<string>('')

  // update pathState
  const updatePath = (b: string[], u: string[], c: string) => {
    const full = [...b, ...u, c].filter(Boolean).join('/')

    setPath(full)
    setName(c) // Update name when path changes
  }

  useEffect(() => {
    const raw = (params as any)?.path
    let newBase: string[] = []

    if (Array.isArray(raw)) {
      newBase = raw.filter(Boolean)
    } else if (typeof raw === 'string') {
      newBase = raw.split('/').filter(Boolean)
    }
    newBase.unshift('')

    setBasePath(newBase)
    updatePath(newBase, userSegments, current)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [JSON.stringify((params as any)?.path)])

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value

    if (value.includes('/')) {
      const rawParts = value.split('/')
      const parts = rawParts.filter(Boolean)
      const last = rawParts[rawParts.length - 1] ?? ''

      if (parts.length > 0) {
        const nextUser = [...userSegments, ...parts]

        setUserSegments(nextUser)
        setCurrent(last)
        updatePath(basePath, nextUser, last)
      } else {
        setCurrent('')
        updatePath(basePath, userSegments, '')
      }
    } else {
      setCurrent(value)
      updatePath(basePath, userSegments, value)
    }
  }

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Backspace' && current === '') {
      e.preventDefault()
      if (userSegments.length > 0) {
        const nextUser = userSegments.slice(0, -1)

        setUserSegments(nextUser)
        updatePath(basePath, nextUser, '')
      } else if (basePath.length > 1) {
        const nextBase = basePath.slice(0, -1)

        setBasePath(nextBase)
        updatePath(nextBase, userSegments, '')
      }
    }
  }

  return (
    <div className='flex max-w-[900px] flex-wrap items-center gap-x-1 gap-y-2 text-gray-700'>
      {[...basePath, ...userSegments].map((seg, i, arr) => (
        // eslint-disable-next-line react/no-array-index-key
        <React.Fragment key={i}>
          <span className='font-medium text-blue-600'>{seg}</span>
          {i < arr.length - 1 && <span>/</span>}
        </React.Fragment>
      ))}
      {[...basePath, ...userSegments].length > 0 && <span>/</span>}

      <input
        type='text'
        value={current}
        placeholder='Name your file...'
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        className='min-w-[140px] rounded border px-2 py-1 outline-none focus:ring-2 focus:ring-blue-500'
        aria-label='file-path-input'
      />
    </div>
  )
}
