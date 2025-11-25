const list = {
  success: true,
  data: {
    items: [
      {
        cl_link: 'cl-100-testing',
        status: 'Testing',
        position: 3,
        display_position: 3,
        created_at: '2025-11-14 12:30:00',
        updated_at: '2025-11-14 12:35:00',
        retry_count: 0,
        error: null
      },
      {
        cl_link: 'cl-200-waiting',
        status: 'Waiting',
        position: 2,
        display_position: 2,
        created_at: '2025-11-14 12:31:00',
        updated_at: '2025-11-14 12:34:00',
        retry_count: 1,
        error: null
      },
      {
        cl_link: 'cl-300-Failed',
        status: 'Failed',
        position: 4,
        display_position: null,
        created_at: '2025-11-14 12:32:00',
        updated_at: '2025-11-14 12:36:00',
        retry_count: 1,
        error: {
          failure_type: 'CI_FAILED',
          message: 'Compilation error: missing import io.gitlab.willsmythe.util.Logger',
          occurred_at: '2025-11-14 17:34:00'
        }
      },
      {
        cl_link: 'cl-400-merging',
        status: 'Merging',
        position: 0,
        display_position: 0,
        created_at: '2025-11-14 12:32:00',
        updated_at: '2025-11-14 12:36:00',
        retry_count: 0,
        error: null
      }
    ],
    total_count: 4
  }
}

const stats = {
  success: true,
  data: {
    stats: {
      total_items: 5,
      waiting_count: 0,
      testing_count: 1,
      merging_count: 2,
      failed_count: 1,
      merged_count: 8
    }
  }
}

const FailedStatus = {
  success: true,
  data: {
    in_queue: true,
    item: {
      cl_link: 'cl-300-failed',
      status: 'Failed',
      position: 1699999800,
      created_at: '2025-11-14 12:20:00',
      updated_at: '2025-11-14 12:25:00',
      retry_count: 0,
      error: {
        failure_type: 'TestFailure',
        message: 'Mock Error: Buck2 tests failed for cl-300-failed',
        occurred_at: '2025-11-14 12:25:00'
      }
    }
  }
}

const MergedStatus = {
  success: true,
  data: {
    in_queue: true,
    item: {
      cl_link: 'cl-500-merged',
      status: 'Merged',
      position: 1699999700,
      created_at: '2025-11-14 12:10:00',
      updated_at: '2025-11-14 12:15:00',
      retry_count: 0,
      error: null
    }
  }
}

const add = {
  success: true,
  data: {
    success: true,
    position: 6,
    message: 'Added to queue'
  }
}

const retry = {
  success: true,
  data: {
    success: true,
    message: 'Item retried'
  }
}

const removeWaiting = {
  success: true,
  data: {
    success: true,
    message: 'Removed from queue'
  }
}

const cancelAll = {
  success: true,
  data: {
    success: true,
    message: 'All pending items cancelled'
  }
}

export { list, stats, FailedStatus, MergedStatus, add, retry, removeWaiting, cancelAll }
