import { atom } from 'jotai'

import { BuildDTO, TargetState, TaskInfoDTO, TaskStatusEnum } from '@gitmono/types/generated'

export enum Status {
  Pending = 'Pending',
  Completed = 'Completed',
  Failed = 'Failed',
  Building = 'Building',
  Interrupted = 'Interrupted',
  NotFound = 'NotFound'
}

export const logsAtom = atom<Record<string, string>>({})
export const statusAtom = atom<Record<string, Status>>({})
export const loadingAtom = atom(true)
export const statusMapAtom = atom<Map<string, BuildDTO>>(new Map())
export const tabAtom = atom<'conversation' | 'check' | 'filechange'>('conversation')

export const mockTasksList: TaskInfoDTO[] = [
  {
    build_list: [
      {
        args: ['--env=prod', '--optimize'],
        created_at: '2025-09-15T08:00:00Z',
        end_at: '2025-09-15T08:20:00Z',
        exit_code: 0,
        id: 'build_1001',
        output_file: 'dist/output_1001.log',
        repo: 'git@github.com:example/frontend.git',
        retry_count: 0,
        start_at: '2025-09-15T08:05:00Z',
        status: TaskStatusEnum.Completed,
        target: 'linux-x64',
        task_id: 'task_1001'
      }
    ],
    created_at: '2025-09-15T07:55:00Z',
    cl_id: 1001,
    task_id: 'task_1001',
    task_name: 'Frontend Build',
    template: 'vue3-template',
    targets: [
      {
        builds: [
          {
            args: ['--env=prod', '--optimize'],
            created_at: '2025-09-15T08:00:00Z',
            end_at: '2025-09-15T08:20:00Z',
            exit_code: 0,
            id: 'build_1001',
            output_file: 'dist/output_1001.log',
            repo: 'git@github.com:example/frontend.git',
            retry_count: 0,
            start_at: '2025-09-15T08:05:00Z',
            status: TaskStatusEnum.Completed,
            target: 'linux-x64',
            task_id: 'task_1001'
          }
        ],
        end_at: '2025-09-15T08:20:00Z',
        id: 'target_1001',
        start_at: '2025-09-15T08:05:00Z',
        state: TargetState.Completed,
        target_path: '//frontend:app'
      }
    ]
  },
  {
    build_list: [
      {
        args: ['--env=staging'],
        created_at: '2025-09-14T11:00:00Z',
        end_at: '2025-09-14T11:25:00Z',
        exit_code: 1,
        id: 'build_2001',
        output_file: 'dist/output_2001.log',
        repo: 'git@github.com:example/backend.git',
        retry_count: 0,
        start_at: '2025-09-14T11:05:00Z',
        status: TaskStatusEnum.Failed,
        target: 'linux-arm64',
        task_id: 'task_2001'
      },
      {
        args: ['--env=staging', '--retry'],
        created_at: '2025-09-14T11:30:00Z',
        end_at: '2025-09-14T11:50:00Z',
        exit_code: 0,
        id: 'build_2002',
        output_file: 'dist/output_2002.log',
        repo: 'git@github.com:example/backend.git',
        retry_count: 1,
        start_at: '2025-09-14T11:32:00Z',
        status: TaskStatusEnum.Interrupted,
        target: 'linux-arm64',
        task_id: 'task_2001'
      }
    ],
    created_at: '2025-09-14T10:50:00Z',
    cl_id: 2001,
    task_id: 'task_2001',
    task_name: 'Backend Build',
    template: 'springboot-template',
    targets: [
      {
        builds: [
          {
            args: ['--env=staging'],
            created_at: '2025-09-14T11:00:00Z',
            end_at: '2025-09-14T11:25:00Z',
            exit_code: 1,
            id: 'build_2001',
            output_file: 'dist/output_2001.log',
            repo: 'git@github.com:example/backend.git',
            retry_count: 0,
            start_at: '2025-09-14T11:05:00Z',
            status: TaskStatusEnum.Failed,
            target: 'linux-arm64',
            task_id: 'task_2001'
          },
          {
            args: ['--env=staging', '--retry'],
            created_at: '2025-09-14T11:30:00Z',
            end_at: '2025-09-14T11:50:00Z',
            exit_code: 0,
            id: 'build_2002',
            output_file: 'dist/output_2002.log',
            repo: 'git@github.com:example/backend.git',
            retry_count: 1,
            start_at: '2025-09-14T11:32:00Z',
            status: TaskStatusEnum.Interrupted,
            target: 'linux-arm64',
            task_id: 'task_2001'
          }
        ],
        end_at: '2025-09-14T11:50:00Z',
        error_summary: 'Build failed on first attempt, succeeded on retry',
        id: 'target_2001',
        start_at: '2025-09-14T11:05:00Z',
        state: TargetState.Interrupted,
        target_path: '//backend:api'
      }
    ]
  },
  {
    build_list: [
      {
        args: ['--env=dev', '--verbose'],
        created_at: '2025-09-13T15:10:00Z',
        end_at: '2025-09-13T15:40:00Z',
        exit_code: 0,
        id: 'build_3001',
        output_file: 'dist/output_3001.log',
        repo: 'git@github.com:example/mobile.git',
        retry_count: 0,
        start_at: '2025-09-13T15:12:00Z',
        status: TaskStatusEnum.Pending,
        target: 'android-arm64',
        task_id: 'task_3001'
      }
    ],
    created_at: '2025-09-13T15:00:00Z',
    cl_id: 3001,
    task_id: 'task_3001',
    task_name: 'Mobile Build',
    template: 'react-native-template',
    targets: [
      {
        builds: [
          {
            args: ['--env=dev', '--verbose'],
            created_at: '2025-09-13T15:10:00Z',
            end_at: '2025-09-13T15:40:00Z',
            exit_code: 0,
            id: 'build_3001',
            output_file: 'dist/output_3001.log',
            repo: 'git@github.com:example/mobile.git',
            retry_count: 0,
            start_at: '2025-09-13T15:12:00Z',
            status: TaskStatusEnum.Pending,
            target: 'android-arm64',
            task_id: 'task_3001'
          }
        ],
        id: 'target_3001',
        start_at: '2025-09-13T15:12:00Z',
        state: TargetState.Pending,
        target_path: '//mobile:app'
      }
    ]
  }
]
