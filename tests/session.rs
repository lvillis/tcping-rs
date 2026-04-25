use std::{
    io::ErrorKind,
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
use tcping::{PingOptions, Target, run_collect_async};

#[tokio::test]
async fn collect_returns_probe_results_and_summary() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();

    let acceptor = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut accepted = 0;

        while accepted < 2 && Instant::now() < deadline {
            match listener.accept() {
                Ok((_stream, _addr)) => accepted += 1,
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("accept failed: {err}"),
            }
        }

        accepted
    });

    let target = Target::new(addr.ip().to_string(), addr.port()).unwrap();
    let options = PingOptions::new(target)
        .with_count(2)
        .unwrap()
        .with_interval(Duration::from_millis(1))
        .with_timeout(Duration::from_millis(500));

    let session = run_collect_async(options).await.unwrap();

    assert_eq!(session.probes.len(), 2);
    assert_eq!(session.summary.total_attempts, 2);
    assert_eq!(session.summary.successful_pings, 2);
    assert_eq!(session.summary.exit_code(), 0);
    assert_eq!(acceptor.join().unwrap(), 2);
}
