import * as Collapsible from '@radix-ui/react-collapsible';
import {
  ArrowDownIcon,
  ArrowUpIcon,
  AlertIcon,
  CheckIcon,
  LoadingSpinner
} from "@gitmono/ui";
import { useEffect, useMemo, useState } from 'react';
import { MergeCheckItem } from './components/MergeCheckItem';
import type { TaskData, AdditionalCheckItem, AdditionalCheckStatus } from './types/mergeCheck.config';
import { ADDITIONAL_CHECK_LABELS } from './types/mergeCheck.config';

interface ChecksSectionProps {
  checks: TaskData[];
  onStatusChange: (hasFailures: boolean) => void;
  additionalChecks?: AdditionalCheckItem[];
}

interface CheckStatusProps {
  hasFailures: boolean;
  failureCount: number;
  inProgressCount: number;
  successCount: number;
  open: boolean;
}

interface CheckGroupProps {
  title: string;
  checks: TaskData[];
}

interface CheckListProps {
  groupedChecks: {
    failing: TaskData[];
    pending: TaskData[];
    success: TaskData[];
  };
}

const CheckStatus = ({ hasFailures, failureCount, inProgressCount, successCount, open }: CheckStatusProps) => {
  let statusInfo: string[] = []

  if (failureCount > 0) {
    statusInfo.push(`${ failureCount } failed`)
  }
  if (inProgressCount > 0) {
    statusInfo.push(`${ inProgressCount } in progress`)
  }
  if (successCount > 0) {
    statusInfo.push(`${ successCount } successful`)
  }
  if (statusInfo.length === 0) {
    statusInfo.push('No checks have run yet')
  }

  return (
    <div className="flex w-full px-3 py-0 items-center">
      { hasFailures? <AlertIcon className="h-5 w-5 mr-3 text-yellow-600"/> :
        <CheckIcon className="h-5 w-5 mr-3 text-green-700"/> }
      <div>
        <p className="font-semibold">
          { hasFailures? 'Some checks were not successful' : 'All checks have passed' }
        </p>
        <p className="text-sm text-gray-600">
          { statusInfo.join(', ') }
        </p>
      </div>
      <button className="justify-self-end ml-auto">
        { open? <ArrowUpIcon/> : <ArrowDownIcon/> }
      </button>
    </div>
  )
}

const CheckGroup = ({ title, checks }: CheckGroupProps) => (
  <div className="mb-2">
    <h4 className="text-xs font-bold text-gray-500 uppercase px-2 py-1">{ title } ({ checks.length })</h4>
    <div>
      { checks.map(check => <MergeCheckItem key={ check.build_id } check={ check }/>) }
    </div>
  </div>
);

const CheckList = ({ groupedChecks }: CheckListProps) => (
  <div className="border-t mt-2 pt-2 max-h-[300px] overflow-y-auto">
    { groupedChecks.failing.length > 0 && <CheckGroup title="Failing" checks={ groupedChecks.failing }/> }
    { groupedChecks.pending.length > 0 && <CheckGroup title="In progress" checks={ groupedChecks.pending }/> }
    { groupedChecks.success.length > 0 && <CheckGroup title="Successful" checks={ groupedChecks.success }/> }
  </div>
)

interface AdditionalCheckItemProps {
  check: AdditionalCheckItem;
}

const getStatusIcon = (status: AdditionalCheckStatus) => {
  switch (status) {
    case 'PASSED':
      return <CheckIcon className="w-4 h-4 text-green-600"/>;
    case 'FAILED':
      return <AlertIcon className="w-4 h-4 text-red-600"/>;
    default:
      return <LoadingSpinner/>;
  }
};

const AdditionalCheckItemComponent = ({ check }: AdditionalCheckItemProps) => (
  <div className="flex items-start px-2 py-2 border-b border-gray-100 last:border-b-0">
    <div className="flex-shrink-0 mr-3 mt-0.5">
      { getStatusIcon(check.result) }
    </div>
    <div className="flex-1 min-w-0">
      <div className="flex items-center justify-between">
        <h5 className="text-sm font-medium text-gray-900">
          { ADDITIONAL_CHECK_LABELS[check.type] }
        </h5>
        <span className={ `text-xs px-2 py-1 rounded-full font-medium ${
          check.result === 'PASSED'? 'bg-green-100 text-green-800' :
            check.result === 'FAILED'? 'bg-red-100 text-red-800' :
              'bg-gray-100 text-gray-800'
        }` }>
          { check.result.toLowerCase() }
        </span>
      </div>
      { check.result === 'FAILED' && (
        <ul className="text-sm text-red-600 mt-1 list-disc list-inside">
          <li className="list-inside">{ check.message }</li>
        </ul>
      )}
    </div>
  </div>
);

interface AdditionalChecksSectionProps {
  additionalChecks: AdditionalCheckItem[];
}

const AdditionalChecksSection = ({ additionalChecks }: AdditionalChecksSectionProps) => {
  if (!additionalChecks || additionalChecks.length === 0) {
    return null;
  }

  return (
    <div className="border-t mt-2 pt-2">
      <h4 className="text-xs font-bold text-gray-500 uppercase px-2 py-1 mb-2">
        Additional Checks ({ additionalChecks.length })
      </h4>
      <div className="space-y-1">
        { additionalChecks.map(check => (
          <AdditionalCheckItemComponent key={ check.message } check={ check }/>
        )) }
      </div>
    </div>
  );
};

export function ChecksSection({ checks, onStatusChange, additionalChecks }: ChecksSectionProps) {
  const summary = useMemo(() => {
    return checks.reduce((acc, check) => {
      acc[check.status] = (acc[check.status] || 0) + 1;
      return acc;
    }, {} as Record<TaskData['status'], number>);
  }, [checks]);

  const failureCount = summary.Failure || 0;
  const inProgressCount = summary.Pending || 0;
  const successCount = summary.Success || 0;
  const hasFailures = failureCount > 0;

  useEffect(() => {
    onStatusChange(hasFailures);
  }, [hasFailures, onStatusChange]);

  const groupedChecks = useMemo(() => {
    const failing = checks.filter(c => c.status === 'Failure');
    const pending = checks.filter(c => c.status === 'Pending');
    const success = checks.filter(c => c.status === 'Success');

    return { failing, pending, success };
  }, [checks]);

  const [open, setOpen] = useState(false);

  return (
    <>
      <Collapsible.Root
        open={ open }
        onOpenChange={ setOpen }
      >
        {/* CheckStatus 部分 */ }
        <Collapsible.Trigger
          className="w-full hover:bg-gray-100 rounded-md cursor-pointer flex"
        >
          <CheckStatus
            hasFailures={ hasFailures }
            failureCount={ failureCount }
            inProgressCount={ inProgressCount }
            successCount={ successCount }
            open={ open }
          />
        </Collapsible.Trigger>

        {/* CheckList & AdditionalChecks 部分 */ }
        <Collapsible.Content>
          <CheckList groupedChecks={ groupedChecks }/>
          <AdditionalChecksSection additionalChecks={ additionalChecks || [] }/>
        </Collapsible.Content>
      </Collapsible.Root>

    </>
  );
}