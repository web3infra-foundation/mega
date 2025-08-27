import * as Collapsible from '@radix-ui/react-collapsible';
import { ArrowDownIcon, ArrowUpIcon } from "@gitmono/ui";
import { useState } from 'react';
import type { ReactNode } from 'react';
import { GroupStatus } from "@/components/MrBox/types/mergeCheck.config";

const groupStatusMap = {
  Pending: { color: 'border-yellow-400' },
  Success: { color: 'border-green-400' },
  Failure: { color: 'border-red-400' },
};

interface CheckGroupProps {
  title: string;
  summary: string;
  status: GroupStatus;
  children: ReactNode;
}

export function CheckGroup({ title, summary, status, children }: CheckGroupProps) {
  const [isOpen, setIsOpen] = useState(true); // 默认展开
  const { color } = groupStatusMap[status];

  return (
    <div className={ `border-l-4 ${ color } rounded` }>
      <Collapsible.Root open={ isOpen } onOpenChange={ setIsOpen }>
        <Collapsible.Trigger className="flex w-full items-center justify-between p-3 bg-gray-50 rounded-t">
          <div className="flex items-center">
            <h4 className="font-bold text-gray-900">{ title }</h4>
            <span className="ml-4 text-sm text-gray-600">{ summary }</span>
          </div>
          { isOpen? <ArrowUpIcon className="h-5 w-5 text-gray-500"/> : <ArrowDownIcon className="h-5 w-5 text-gray-500"/> }
        </Collapsible.Trigger>
        <Collapsible.Content className="p-2 bg-white rounded-b">
          { children }
        </Collapsible.Content>
      </Collapsible.Root>
    </div>
  );
}