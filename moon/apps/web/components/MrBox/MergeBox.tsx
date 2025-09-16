import React, { useState, useCallback } from 'react';
import { useMergeChecks } from './hooks/useMergeChecks';
import { ReviewerSection } from './ReviewerSection';
import { ChecksSection } from './ChecksSection';
import { MergeSection } from './MergeSection';
import { FeedMergedIcon } from "@primer/octicons-react";
import { usePostMrMerge } from "@/hooks/usePostMrMerge";
import { useGetMergeBox } from "@/components/MrBox/hooks/useGetMergeBox";
import { LoadingSpinner } from "@gitmono/ui";

const REQUIRED_REVIEWERS = 2; // 假设需要2个 reviewer

export function MergeBox({ prId }: { prId: string }) {
  const { checks, refresh } = useMergeChecks(prId);
  const [isReviewerApproved, setIsReviewerApproved] = useState(false);
  const [hasCheckFailures, setHasCheckFailures] = useState(true);
  const { mutate: approveMr, isPending: mrMergeIsPending } = usePostMrMerge(prId)

  const { mergeBoxData, isLoading } = useGetMergeBox(prId)

  // 定义最终的合并处理函数
  const handleMerge = useCallback(async () => {
    // console.log('Final validation before merge...');
    // TODO: 再次发送校验请求
    refresh();

    // 模拟校验结果
    const stillHasFailures = false;

    if (stillHasFailures) {
      alert("阻止合并：仍有检查项未通过，请刷新页面查看详情。");
    } else {
      // console.log('All checks passed. Sending merge request to backend...');

      approveMr(undefined)

      alert("合并请求已发送！");
    }
  }, [approveMr, refresh]);

  const additionalChecks = mergeBoxData?.merge_requirements?.conditions ?? []

  return (
    <div className="flex">
      <FeedMergedIcon size={ 24 } className='text-gray-500 ml-1'/>
      { isLoading? (
        <div className='flex h-[400px] items-center justify-center'>
          <LoadingSpinner/>
        </div>
      ) : (
        <div className="border rounded-lg bg-white divide-y ml-3 w-full">
          <ReviewerSection
            required={ REQUIRED_REVIEWERS }
            onStatusChange={ setIsReviewerApproved }
          />
          <ChecksSection
            checks={ checks }
            onStatusChange={ setHasCheckFailures }
            additionalChecks={ additionalChecks }
          />
          <MergeSection
            isReviewerApproved={ isReviewerApproved }
            hasCheckFailures={ hasCheckFailures }
            onMerge={ handleMerge }
            isMerging={ mrMergeIsPending }
          />
        </div>
      ) }
    </div>
  );
}