use orion_server::scheduler::{create_log_file, read_log_segment_raw, LogReadError};
use std::io::Write;
use once_cell::sync::Lazy;
use std::sync::Mutex;

static TEST_LOG_DIR: Lazy<Mutex<Option<tempfile::TempDir>>> = Lazy::new(|| Mutex::new(None));

fn init_log_dir() {
    let mut guard = TEST_LOG_DIR.lock().unwrap();
    if guard.is_none() {
        let tmp = tempfile::tempdir().expect("temp dir");
        unsafe { std::env::set_var("BUILD_LOG_DIR", tmp.path().to_str().unwrap()); }
        *guard = Some(tmp);
    }
}

fn write_log(task_id: &str, content: &str) {
    init_log_dir();
    let mut f = create_log_file(task_id).expect("create log file");
    write!(f, "{content}").unwrap();
}

fn create_empty_log(task_id: &str) { init_log_dir(); let _ = create_log_file(task_id).unwrap(); }

#[tokio::test]
async fn test_log_segment_read_basic() {
    write_log("segment-basic", "Hello Log Segment Test!");
    let seg = read_log_segment_raw("segment-basic", 0, 5).await.expect("segment");
    assert_eq!(seg.data, "Hello");
    let seg2 = read_log_segment_raw("segment-basic", seg.next_offset, 1024).await.expect("segment2");
    assert!(seg2.data.starts_with(" Log Segment"));
}

#[tokio::test]
async fn test_log_segment_offset_out_of_range() {
    create_empty_log("segment-oob");
    let res = read_log_segment_raw("segment-oob", 10, 10).await;
    assert!(matches!(res, Err(LogReadError::OffsetOutOfRange { .. })));
}

#[tokio::test]
async fn test_log_segment_zero_len_metadata() {
    write_log("segment-zero", "1234567890");
    let seg = read_log_segment_raw("segment-zero", 0, 0).await.expect("segment");
    assert_eq!(seg.len, 0);
    assert_eq!(seg.file_size, 10);
    assert!(!seg.eof);
}
