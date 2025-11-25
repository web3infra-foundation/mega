import React from 'react'
import { GitPullRequestDraftIcon } from '@primer/octicons-react'

import { useUpdateClStatus } from '@/components/ClView/hook/useUpdateClStatus'

interface DraftStatusBannerProps {
  link: string
}

export const DraftStatusBanner: React.FC<DraftStatusBannerProps> = ({ link }) => {
  const { mutate: updateStatus, isPending } = useUpdateClStatus()

  const handleReadyForReview = () => {
    updateStatus({ link, status: 'Open' })
  }

  return (
    <div className='flex items-center justify-between px-4 py-3'>
      <div className='flex items-start gap-3'>
        <div className='mt-1'>
          <GitPullRequestDraftIcon className='text-[#6e7781]' size={24} />
        </div>
        <div>
          <div className='text-sm font-semibold text-[#24292f]'>This pull request is still a work in progress</div>
          <div className='mt-1 text-xs text-[#57606a]'>Draft pull requests cannot be merged.</div>
        </div>
      </div>
      <button
        type='button'
        onClick={handleReadyForReview}
        disabled={isPending}
        className='rounded-md border border-[#d0d7de] bg-[#f6f8fa] px-3 py-1 text-sm font-semibold text-[#24292f] hover:bg-[#eef1f4] disabled:cursor-not-allowed disabled:opacity-60'
      >
        Ready for review
      </button>
    </div>
  )
}
