import {CheckCircleIcon, AlertIcon, WarningTriangleIcon} from "@gitmono/ui";
import {TaskData} from "../types/mergeCheck.config";

const statusMap = {
  Pending: { Icon: WarningTriangleIcon, className: 'text-yellow-600' },
  Success: { Icon: CheckCircleIcon, className: 'text-green-600' },
  Failure: { Icon: AlertIcon, className: 'text-red-600' },
  Warning: { Icon: WarningTriangleIcon, className: 'text-yellow-600' }, // GitHub 通常用黄点表示 in-progress 或 warning
};

export function MergeCheckItem({ check }: { check: TaskData }) {
  const { Icon, className } = statusMap[check.status];

  return (
    <div className="flex items-center p-2 hover:bg-gray-100 rounded-md">
      <Icon className={`h-5 w-5 flex-shrink-0 ${className}`} />
      <div className="ml-3 flex-grow">
        <span className="font-semibold text-gray-800">{check.repo_name}</span>
        {check.arguments && <span className="ml-2 text-gray-500 text-sm">{check.arguments}</span>}
      </div>
      <button className="text-gray-500 hover:text-gray-800">

      </button>
    </div>
  );
}