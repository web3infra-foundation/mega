'use client'

import { Flex, Button } from '@radix-ui/themes'
import { TextField } from '@gitmono/ui/TextField'

import {GitPullRequestIcon,RocketIcon,PaperAirplaneIcon} from '@primer/octicons-react'
const CodeViewHeader = () => {
  return (
    <div className="w-full p-6 space-y-4">
      {/* Ask Copilot Input */}
      <div className="relative max-w-8xl">
        <TextField
          placeholder="Ask Copilot"
          additionalClasses="w-full h-13 pl-4 pr-12 rounded-lg border border-gray-300 bg-white text-gray-900 placeholder-gray-500 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
        <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
          <Button
            variant="ghost"
            size="1"
            className="p-1 hover:bg-gray-100 rounded "
          >
            <PaperAirplaneIcon className="h-5 w-5 text-gray-400" />
          </Button>
        </div>
      </div>

      {/* Action Buttons */}
      <Flex gap="3" className="w-full max-w-4xl">
        <Button
          variant="soft"
          size="3"
          className="flex-1 !h-12 !bg-white hover:!bg-gray-100 !border !border-solid !border-gray-300 !rounded-2xl !text-black"
        >
          <Flex align="center" gap="2">
            <GitPullRequestIcon className="h-4 w-4 text-[#378f50]" />
            <span>Summarize a pull request</span>
          </Flex>
        </Button>

        <Button
          variant="soft"
          size="3"
          className="flex-1 !h-12 !bg-white hover:!bg-gray-100 !border !border-solid !border-gray-300 !rounded-2xl !text-black"
        >
          <Flex align="center" gap="2">
            <RocketIcon className="h-4 w-4 text-[#A33A77]" />
            <span>Create a profile README for me</span>
          </Flex>
        </Button>

        <Button
          variant="soft"
          size="3"
          className="flex-1 !h-12 !bg-white hover:!bg-gray-100 !border !border-solid !border-gray-300 !rounded-2xl !text-black"
        >
          <Flex align="center" gap="2">
            <GitPullRequestIcon className="h-4 w-4 text-[#378f50]" />
            <span>My open pull requests</span>
          </Flex>
        </Button>

      </Flex>
    </div>
  )
}

export default CodeViewHeader
