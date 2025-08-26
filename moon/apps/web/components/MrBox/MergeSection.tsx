import { AlertIcon, WarningTriangleIcon, CheckCircleIcon, LoadingSpinner } from "@gitmono/ui";
import React from "react";

interface MergeSectionProps {
  isReviewerApproved: boolean;
  hasCheckFailures: boolean;
  onMerge: () => Promise<void>;
  isMerging: boolean;
}

export function MergeSection({ isReviewerApproved, hasCheckFailures, onMerge, isMerging }: MergeSectionProps) {
  let statusNode: React.ReactNode;
  const isMergeable = isReviewerApproved && !hasCheckFailures;

  if (!isReviewerApproved) {
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
      <button
        onClick={onMerge}
        disabled={!isMergeable}
        className="w-full mt-3 px-4 py-2 font-bold text-white bg-green-600 rounded-md
                   hover:bg-green-700
                   disabled:bg-gray-400 disabled:cursor-not-allowed"
      >
        {isMerging ? <LoadingSpinner/> : "Confirm Merge"}
      </button>
    </div>
  );
}