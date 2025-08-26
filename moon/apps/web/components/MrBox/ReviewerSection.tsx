import { useState, useEffect } from 'react';
import { CheckCircleIcon, AlertIcon } from "@gitmono/ui";

interface ReviewerSectionProps {
  required: number;
  onStatusChange: (isApproved: boolean) => void;
}

export function ReviewerSection({ required, onStatusChange }: ReviewerSectionProps) {
  // 实际的 review 人数，暂时用 state 模拟，未来应由 API 获取
  const [actual, _setActual] = useState(1);
  const isApproved = actual >= required;

  useEffect(() => {
    // TODO: 调用 API 获取实际 review 人数并更新 setActual
    // fetch('/api/.../reviewers').then(res => res.json()).then(data => setActual(data.count));
  }, []);

  useEffect(() => {
    onStatusChange(isApproved);
  }, [isApproved, onStatusChange]);

  if (isApproved) {
    return (
      <div className="flex items-center p-3 text-green-700">
        <CheckCircleIcon className="h-5 w-5 mr-3" />
        <div>
          <span className="font-semibold">All required reviewers have approved</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center p-3 text-gray-800">
      <AlertIcon className="h-5 w-5 mr-3 text-yellow-600" />
      <div>
        <div className="font-semibold">Review required</div>
        <div className="ml-auto text-sm text-gray-500">
          {`At least ${required} reviewer${required > 1 ? 's' : ''} required with write access.`}
        </div>
      </div>
    </div>
  );
}