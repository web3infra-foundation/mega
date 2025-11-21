import { CheckType, Condition, ConditionResult } from '@gitmono/types'

export interface TaskData {
  arguments: string
  build_id: string
  end_at: string
  exit_code: number
  cl: string
  output_file: string
  repo_name: string
  start_at: string
  status: 'Pending' | 'Success' | 'Failure' | 'Warning'
  target: string
}

export type GetApiTasksData = TaskData[]

export type GroupStatus = 'Success' | 'Failure' | 'Pending'

export type AdditionalCheckStatus = ConditionResult

export type AdditionalCheckItem = Condition

export type AdditionalCheckType = CheckType

export const ADDITIONAL_CHECK_LABELS: Record<AdditionalCheckType, string> = {
  [CheckType.GpgSignature]: 'GPG Signature Verification',
  [CheckType.BranchProtection]: 'Branch Protection',
  [CheckType.CommitMessage]: 'Commit Message Format',
  [CheckType.CiStatus]: 'CI Status',
  [CheckType.ClSync]: 'CL Sync Status',
  [CheckType.MergeConflict]: 'Merge Conflict Detection',
  [CheckType.CodeReview]: 'Code Review Status'
}
