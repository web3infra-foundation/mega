import { CheckType, Condition, ConditionResult } from "@gitmono/types";

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

export type AdditionalCheckStatus = ConditionResult;

export type AdditionalCheckItem = Condition

export type AdditionalCheckType = CheckType

export const ADDITIONAL_CHECK_LABELS: Record<AdditionalCheckType, string> = {
  [CheckType.GpgSignature]: 'GPG签名验证',
  [CheckType.BranchProtection]: '分支保护',
  [CheckType.CommitMessage]: '提交信息规范',
  [CheckType.CiStatus]: 'CI状态',
  [CheckType.MrSync]: 'Mr同步状态',
  [CheckType.MergeConflict]: '合并冲突检测',
  [CheckType.CodeReview]: '代码审查状态'
};