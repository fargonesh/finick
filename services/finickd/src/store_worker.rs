use system::store::{self, StoreRequest, StoreResponse};

/// Serve one store request: forward Started/Log, run the job blocking
/// (start_server already gives us a per-connection thread), send Finished.
pub fn handle_store_request(req: StoreRequest, sender: std::sync::mpsc::Sender<StoreResponse>) {
    let StoreRequest::Run { job } = req;
    let mut sender_opt = Some(sender);
    let res = store::run_job(job, &mut |ev| {
        let resp = match ev {
            store::StoreProgress::Started(job) => StoreResponse::Started { job },
            store::StoreProgress::Log(line) => StoreResponse::Log(line),
        };
        if sender_opt.as_ref().is_some_and(|s| s.send(resp).is_err()) {
            sender_opt = None;
        }
    });
    if let Some(sender) = sender_opt {
        let _ = sender.send(StoreResponse::Finished { result: res });
    }
}
