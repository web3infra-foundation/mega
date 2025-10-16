import { CheckCircleIcon, AlertIcon } from "@gitmono/ui";

interface ReviewerSectionProps {
  required: number;
  actual: number;
}

export function ReviewerSection({ required, actual }: ReviewerSectionProps) {
  const isApproved = actual >= required;

  if (isApproved) {
    return (
      <div className="flex items-center p-3 text-green-700">
        <CheckCircleIcon className="h-5 w-5 mr-3"/>
        <div>
          <span className="font-semibold">All required reviewers have approved</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center p-3 text-gray-800">
      <AlertIcon className="h-5 w-5 mr-3 text-yellow-600"/>
      <div>
        <div className="font-semibold">Review required</div>
        <div className="ml-auto text-sm text-gray-500">
          { `At least ${ required } reviewer${ required > 1? 's' : '' } required with write access, now has ${ actual }` }
        </div>
      </div>
    </div>
  );
}