use super::InternalEvent;
use metrics::counter;

#[derive(Debug)]
pub struct ElasticSearchEventReceived {
    pub byte_size: usize,
    pub index: String,
}

impl InternalEvent for ElasticSearchEventReceived {
    fn emit_logs(&self) {
        trace!(message = "Inserting event.", index = %self.index);
    }

    fn emit_metrics(&self) {
        counter!("events_processed_total", 1);
        counter!("processed_bytes_total", self.byte_size as u64);
    }
}

#[derive(Debug)]
pub struct ElasticSearchMissingKeys<'a> {
    pub keys: &'a [String],
}

impl<'a> InternalEvent for ElasticSearchMissingKeys<'a> {
    fn emit_logs(&self) {
        warn!(
            message = "Keys do not exist on the event; dropping event.",
            missing_keys = ?self.keys,
            rate_limit_secs = 30,
        )
    }

    fn emit_metrics(&self) {
        counter!("missing_keys_total", 1);
    }
}
