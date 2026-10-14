use omamusi::background::BackgroundTasks;
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

fn poll<T: Send + 'static>(tasks: &BackgroundTasks<T>) -> Vec<omamusi::background::TaskResult<T>> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let results = tasks.poll();
        if !results.is_empty() {
            return results;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn one_running_one_replaceable_pending_and_only_latest_result_delivered() {
    let tasks = BackgroundTasks::new();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    tasks.submit("metadata", move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        Ok(1)
    });
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let (executed_tx, executed_rx) = mpsc::channel();
    tasks.submit("metadata", move || {
        executed_tx.send(()).unwrap();
        Ok(2)
    });
    let latest = tasks.submit("metadata", || Ok(3)).unwrap();
    release_tx.send(()).unwrap();
    let results = poll(&tasks);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].generation, latest);
    assert_eq!(results[0].category, "metadata");
    assert_eq!(*results[0].result.as_ref().unwrap(), 3);
    assert!(executed_rx.recv_timeout(Duration::from_millis(20)).is_err());
}

#[test]
fn invalidation_discards_running_results_and_pending_tasks() {
    let tasks = BackgroundTasks::new();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    tasks.submit("scan", move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        done_tx.send(()).unwrap();
        Ok(1)
    });
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    tasks.submit("scan", || Ok(2));
    tasks.invalidate("scan");
    release_tx.send(()).unwrap();
    done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    tasks.submit("other", || Ok(4));
    let results = poll(&tasks);
    assert!(results.iter().all(|r| r.category == "other"));
    assert!(tasks.poll().is_empty());
}

#[test]
fn close_does_not_wait_for_blocked_filesystem_work() {
    let tasks = BackgroundTasks::<u32>::new();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    tasks.submit("filesystem", move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        Ok(1)
    });
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let start = Instant::now();
    tasks.close();
    assert!(start.elapsed() < Duration::from_millis(100));
    assert!(tasks.submit("new", || Ok(2)).is_none());
    assert!(tasks.poll().is_empty());
    drop(tasks);
    release_tx.send(()).unwrap();
}

#[test]
fn worker_error_and_panic_are_delivered_without_breaking_future_requests() {
    let tasks = BackgroundTasks::<u32>::new();
    tasks.submit("scan", || anyhow::bail!("unreadable"));
    assert!(
        poll(&tasks)[0]
            .result
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("unreadable")
    );
    tasks.submit("scan", || panic!("broken worker"));
    assert!(poll(&tasks)[0].result.is_err());
    tasks.submit("scan", || Ok(7));
    assert_eq!(*poll(&tasks)[0].result.as_ref().unwrap(), 7);
}
