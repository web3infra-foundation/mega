import React from 'react'
import Image from 'next/image'
import { useGetCommitBinding } from '@/hooks/useGetCommitBinding'

interface CommitMessageWithAuthorProps {
  commitMessage: string
  commitSha?: string
}

const CommitMessageWithAuthor: React.FC<CommitMessageWithAuthorProps> = ({ 
  commitMessage, 
  commitSha 
}) => {
  const { data: commitBinding, isLoading, error } = useGetCommitBinding(commitSha)

  return (
    <div className="flex flex-col space-y-1">
      <a className='cursor-pointer transition-colors duration-300 text-gray-600 hover:text-[#69b1ff] text-sm'>
        {commitMessage}
      </a>
      <div className="flex items-center space-x-2 text-xs">
        {isLoading ? (
          <span className="text-gray-400">加载中...</span>
        ) : commitBinding ? (
          <>
            {/* User Avatar */}
            {commitBinding.avatar_url && (
              <Image 
                src={commitBinding.avatar_url} 
                alt={commitBinding.display_name}
                width={16}
                height={16}
                className="rounded-full"
                onError={(e) => {
                  // Hide image on error
                  e.currentTarget.style.display = 'none'
                }}
              />
            )}
            
            <span className="text-gray-500">
              by <span className="font-medium">{commitBinding.display_name}</span>
            </span>
            
            {/* Status Indicators */}
            {commitBinding.is_anonymous && (
              <span className="px-1.5 py-0.5 bg-orange-100 text-orange-600 rounded-full text-xs font-medium">
                匿名
              </span>
            )}
            
            {commitBinding.is_verified_user && (
              <span className="px-1.5 py-0.5 bg-green-100 text-green-600 rounded-full text-xs font-medium">
                已验证
              </span>
            )}
          </>
        ) : error ? (
          <span className="text-gray-400">by 匿名提交</span>
        ) : null}
      </div>
    </div>
  )
}

export default CommitMessageWithAuthor
