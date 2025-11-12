'use client'

import React, { useState } from 'react'
import BuildIcon from '@mui/icons-material/Build'
import { DiscussionClosedIcon, HashIcon, ImageIcon, MentionIcon } from '@primer/octicons-react'
import { isWindows } from 'react-device-detect'

import { ArrowUpIcon, Button, ChevronDownIcon, MicrophoneIcon, SparklesIcon, UIText } from '@gitmono/ui'

export function WorkWithChatDialog() {
  const hintText = `'↑↓' to navigate input history, '${isWindows ? 'Ctrl' : '⌘'}↲' to insert a new line`
  const [message, setMessage] = useState(hintText)
  const [isShowingHint, setIsShowingHint] = useState(true)
  const [autoMode] = useState('Auto')

  const handleFocus = () => {
    if (isShowingHint) {
      setMessage('')
      setIsShowingHint(false)
    }
  }

  const handleBlur = () => {
    if (message.trim() === '') {
      setMessage(hintText)
      setIsShowingHint(true)
    }
  }

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const value = e.target.value

    setMessage(value)
    if (isShowingHint && value !== hintText) {
      setIsShowingHint(false)
    }
  }

  return (
    <div className='w-full rounded-lg bg-gray-100'>
      {/* Top Icon and Title - White background */}
      <div className='mb-0 flex flex-col items-center rounded-t-lg p-4' style={{ backgroundColor: '#ffffff' }}>
        <div
          className='mb-3 flex h-16 w-16 items-center justify-center rounded-lg'
          style={{ backgroundColor: '#52b5f2' }}
        >
          <DiscussionClosedIcon size={24} className='text-black' />
        </div>
        <UIText size='text-lg' weight='font-medium' className='text-gray-700'>
          Work with <span style={{ color: '#111214' }}>Chat</span>
        </UIText>
      </div>

      {/* Input box with border and full width */}
      <div className='w-full rounded-lg border border-gray-300' style={{ backgroundColor: '#f3f4f5' }}>
        {/* @Chat header section with rounded top corners */}
        <div className='flex items-center gap-2 rounded-t-lg px-3 py-2' style={{ backgroundColor: '#e6e8ea' }}>
          <div className='flex h-6 w-6 items-center justify-center rounded' style={{ backgroundColor: '#52b5f2' }}>
            <DiscussionClosedIcon size={14} className='text-black' />
          </div>
          <UIText size='text-sm' weight='font-medium' className='text-gray-700'>
            @Chat
          </UIText>
          <Button
            variant='plain'
            iconOnly={<BuildIcon style={{ fontSize: 14 }} />}
            accessibilityLabel='Build settings'
            className='h-5 w-5 p-0 text-gray-600'
          />
        </div>

        {/* Input area */}
        <div className='relative min-h-[100px] p-3'>
          {/* Text area - takes most of the space */}
          <div className='mb-10'>
            <textarea
              value={message}
              onChange={handleChange}
              onFocus={handleFocus}
              onBlur={handleBlur}
              placeholder=''
              className='w-full resize-none border-0 bg-transparent text-sm focus:outline-none focus:ring-0'
              style={{
                minHeight: '60px',
                lineHeight: '1.4',
                color: isShowingHint ? '#8d9297' : '#374151'
              }}
              rows={3}
            />
          </div>

          {/* Bottom toolbar */}
          <div className='absolute bottom-3 left-3 right-3 flex items-center justify-between'>
            {/* Left side icons */}
            <div className='flex items-center gap-2'>
              <Button
                variant='plain'
                iconOnly={<MentionIcon size={16} />}
                accessibilityLabel='Mention'
                className='h-6 w-6 p-0 text-gray-600 hover:text-gray-800'
              />
              <Button
                variant='plain'
                iconOnly={<HashIcon size={16} />}
                accessibilityLabel='Add hashtag'
                className='h-6 w-6 p-0 text-gray-600 hover:text-gray-800'
              />
              <Button
                variant='plain'
                iconOnly={<ImageIcon size={16} />}
                accessibilityLabel='Add image'
                className='h-6 w-6 p-0 text-gray-600 hover:text-gray-800'
              />
            </div>

            {/* Right side buttons */}
            <div className='flex items-center gap-2'>
              {/* Auto dropdown with green dot */}
              <div className='relative flex items-center'>
                <Button
                  variant='plain'
                  size='sm'
                  className='h-6 px-2 text-xs text-gray-600'
                  rightSlot={<ChevronDownIcon size={12} />}
                >
                  {autoMode}
                </Button>
                <div className='absolute -right-1 -top-1 h-2 w-2 rounded-full bg-green-500' />
              </div>

              {/* Magic wand icon */}
              <Button
                variant='plain'
                iconOnly={<SparklesIcon size={16} />}
                accessibilityLabel='AI assistant'
                className='h-6 w-6 p-0 text-gray-600 hover:text-gray-800'
              />

              {/* Microphone icon */}
              <div
                className='flex h-6 w-6 cursor-pointer items-center justify-center rounded-md hover:opacity-80'
                style={{ backgroundColor: '#e2e4e7' }}
              >
                <MicrophoneIcon size={16} style={{ color: '#45484d' }} />
              </div>

              {/* Send button */}
              <div
                className='flex h-6 w-6 cursor-pointer items-center justify-center rounded-md hover:opacity-80'
                style={{ backgroundColor: '#d2e6d8' }}
              >
                <ArrowUpIcon size={16} strokeWidth='2.5' style={{ color: '#101011' }} />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
