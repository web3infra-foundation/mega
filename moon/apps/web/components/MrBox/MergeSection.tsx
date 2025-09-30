import { AlertIcon, WarningTriangleIcon, CheckCircleIcon, LoadingSpinner } from "@gitmono/ui";
import React from "react";

interface MergeSectionProps {
  isNowUserApprove?: boolean;
  isAllReviewerApproved: boolean;
  hasCheckFailures: boolean;
  onMerge: () => Promise<void>;
  onApprove: () => void;
  isMerging: boolean;
}

export function MergeSection({ isAllReviewerApproved, hasCheckFailures, isNowUserApprove, onMerge, onApprove, isMerging }: MergeSectionProps) {
  let statusNode: React.ReactNode;
  const isMergeable = isAllReviewerApproved && !hasCheckFailures;

  if (!isAllReviewerApproved) {
    statusNode = (
      <div className="flex items-center text-yellow-700">
        <WarningTriangleIcon className="h-5 w-5 mr-3" />
        <span className="font-semibold">Merging is blocked</span>
      </div>
    );
  } else if (hasCheckFailures) {
    statusNode = (
      <div className="flex items-center text-red-700">
        <AlertIcon className="h-5 w-5 mr-3" />
        <span className="font-semibold">Merging is blocked</span>
      </div>
    );
  } else {
    statusNode = (
      <div className="flex items-center text-green-700">
        <CheckCircleIcon className="h-5 w-5 mr-3" />
        <span className="font-semibold">Allow merging</span>
      </div>
    );
  }

  return (
    <div className="p-3">
      {statusNode}
      <div className="flex items-center justify-center space-x-4">
        <button
          onClick={onApprove}
          disabled={isNowUserApprove === undefined || isNowUserApprove}
          className="w-full mt-3 px-4 py-2 font-bold text-white bg-green-600 rounded-md
                   hover:bg-green-800
                   duration-500
                   disabled:bg-gray-400 disabled:cursor-not-allowed"
        >
          {"Approve"}
        </button>
        <button
          onClick={onMerge}
          disabled={!isMergeable}
          className="w-full mt-3 px-4 py-2 font-bold text-white bg-green-600 rounded-md
                   hover:bg-green-800
                   duration-500
                   disabled:bg-gray-400 disabled:cursor-not-allowed"
        >
          {isMerging ? <LoadingSpinner/> : "Confirm Merge"}
        </button>
      </div>
    </div>
  );
}