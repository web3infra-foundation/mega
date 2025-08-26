export interface TaskData {
  "arguments": string,
  "build_id": string,
  "end_at": string,
  "exit_code": number,
  "mr": string,
  "output_file": string,
  "repo_name": string,
  "start_at": string,
  "status": "Pending" | "Success" | "Failure" | "Warning",
  "target": string
}

export type GetApiTasksData = TaskData[]

export type GroupStatus = 'Success' | 'Failure' | 'Pending';

export type AdditionalCheckStatus = 'Success' | 'Failure' | 'Pending' | 'Warning';

export interface AdditionalCheckItem {
  type: AdditionalCheckType;
  status: AdditionalCheckStatus;
  message?: string;
  errors?: string[];
}

export type AdditionalCheckType = 
  | 'GPG_SIGNATURE'
  | 'BRANCH_PROTECTION'
  | 'COMMIT_MESSAGE_FORMAT'
  | 'PR_UPDATE_STATUS'
  | 'CONFLICT_DETECTION';

export const ADDITIONAL_CHECK_LABELS: Record<AdditionalCheckType, string> = {
  GPG_SIGNATURE: 'GPG 签名验证',
  BRANCH_PROTECTION: '分支保护规则',
  COMMIT_MESSAGE_FORMAT: '提交信息规范',
  PR_UPDATE_STATUS: 'PR 更新状态',
  CONFLICT_DETECTION: '冲突检测'
};