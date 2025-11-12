'use client'

import { GitPullRequestIcon, PaperAirplaneIcon, RocketIcon } from '@primer/octicons-react'
import { Button, Flex } from '@radix-ui/themes'

import { TextField } from '@gitmono/ui/TextField'

const CodeViewHeader = () => {
  return (
    <div className='w-full space-y-4 p-6'>
      {/* Ask Copilot Input */}
      <div className='max-w-8xl relative'>
        <TextField
          placeholder='Ask Copilot'
          additionalClasses='w-full h-13 pl-4 pr-12 rounded-lg border border-gray-300 bg-white text-gray-900 placeholder-gray-500 focus:ring-2 focus:ring-blue-500 focus:border-transparent'
        />
        <div className='absolute right-3 top-1/2 -translate-y-1/2 transform'>
          <Button variant='ghost' size='1' className='rounded p-1 hover:bg-gray-100'>
            <PaperAirplaneIcon className='h-5 w-5 text-gray-400' />
          </Button>
        </div>
      </div>

      {/* Action Buttons */}
      <Flex gap='3' className='w-full max-w-4xl' style={{ marginTop: '16px' }}>
        <Button
          variant='soft'
          size='3'
          className='!h-12 flex-1 !rounded-2xl !border !border-solid !border-gray-300 !bg-white !text-black hover:!bg-gray-100'
        >
          <Flex align='center' gap='2'>
            <GitPullRequestIcon className='h-4 w-4 text-[#378f50]' />
            <span>Summarize a pull request</span>
          </Flex>
        </Button>

        <Button
          variant='soft'
          size='3'
          className='!h-12 flex-1 !rounded-2xl !border !border-solid !border-gray-300 !bg-white !text-black hover:!bg-gray-100'
        >
          <Flex align='center' gap='2'>
            <RocketIcon className='h-4 w-4 text-[#A33A77]' />
            <span>Create a profile README for me</span>
          </Flex>
        </Button>

        <Button
          variant='soft'
          size='3'
          className='!h-12 flex-1 !rounded-2xl !border !border-solid !border-gray-300 !bg-white !text-black hover:!bg-gray-100'
        >
          <Flex align='center' gap='2'>
            <GitPullRequestIcon className='h-4 w-4 text-[#378f50]' />
            <span>My open change lists</span>
          </Flex>
        </Button>
      </Flex>
    </div>
  )
}

export default CodeViewHeader
