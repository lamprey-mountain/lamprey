use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering::Relaxed},
    },
    time::Duration,
};

use opentelemetry::global;
use str0m::{
    media::{Mid, Rid},
    stats::{MediaEgressStats, MediaIngressStats, PeerStats},
};

/// aggregate, lock-free counters for one shard
#[derive(Default)]
pub struct ShardMetrics {
    peers: AtomicU64,
    calls: AtomicU64,
    bytes_rx: AtomicU64,
    bytes_tx: AtomicU64,
    packets_rx: AtomicU64,
    packets_tx: AtomicU64,
    nacks: AtomicU64,
    plis: AtomicU64,
    firs: AtomicU64,
    bwe_tx_bps: AtomicU64,
    rtt_us_sum: AtomicU64,
    rtt_count: AtomicU64,
    loss_ppm_sum: AtomicU64,
    loss_count: AtomicU64,
    jitter_sum: AtomicU64,
    jitter_count: AtomicU64,
    udp_send_errors: AtomicU64,
}

impl ShardMetrics {
    pub fn peer_created(&self) {
        self.peers.fetch_add(1, Relaxed);
    }

    pub fn peer_removed(&self, bwe_tx: u64) {
        self.peers.fetch_sub(1, Relaxed);
        if bwe_tx > 0 {
            self.bwe_tx_bps.fetch_sub(bwe_tx, Relaxed);
        }
    }

    pub fn call_created(&self) {
        self.calls.fetch_add(1, Relaxed);
    }

    pub fn udp_send_error(&self) {
        self.udp_send_errors.fetch_add(1, Relaxed);
    }

    fn update_bwe(&self, old: u64, new: u64) {
        if new >= old {
            self.bwe_tx_bps.fetch_add(new - old, Relaxed);
        } else {
            self.bwe_tx_bps.fetch_sub(old - new, Relaxed);
        }
    }

    fn observe_rtt(&self, rtt: Duration) {
        self.rtt_us_sum.fetch_add(rtt.as_micros() as u64, Relaxed);
        self.rtt_count.fetch_add(1, Relaxed);
    }

    fn observe_loss(&self, loss: f32) {
        let ppm = (loss.clamp(0.0, 1.0) * 1_000_000.0) as u64;
        self.loss_ppm_sum.fetch_add(ppm, Relaxed);
        self.loss_count.fetch_add(1, Relaxed);
    }

    fn observe_jitter(&self, jitter: u32) {
        self.jitter_sum.fetch_add(u64::from(jitter), Relaxed);
        self.jitter_count.fetch_add(1, Relaxed);
    }
}

/// process-wide metrics registry and observable instruments
#[derive(Default)]
pub struct VoiceMetrics {
    shards: Mutex<Vec<Arc<ShardMetrics>>>,
    active_shards: AtomicU64,
}

impl VoiceMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_shard(&self) -> Arc<ShardMetrics> {
        let metrics = Arc::new(ShardMetrics::default());
        self.lock_shards().push(Arc::clone(&metrics));
        metrics
    }

    pub fn set_active_shards(&self, active: u64) {
        self.active_shards.store(active, Relaxed);
    }

    pub fn shard_stopped(&self) {
        self.active_shards.fetch_sub(1, Relaxed);
    }

    pub fn register(self: &Arc<Self>) {
        let meter = global::meter("lamprey-voice");

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_gauge("voice.shards.active")
            .with_description("Number of active voice shards")
            .with_callback(move |observer| {
                observer.observe(metrics.active_shards.load(Relaxed), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_gauge("voice.calls.active")
            .with_description("Number of active voice calls")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.calls.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_gauge("voice.peers.active")
            .with_description("Number of active voice peers")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.peers.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_counter("voice.rtp.bytes_rx_total")
            .with_description("Media bytes received")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.bytes_rx.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_counter("voice.rtp.bytes_tx_total")
            .with_description("Media bytes transmitted")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.bytes_tx.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_counter("voice.rtp.packets_rx_total")
            .with_description("RTP packets received")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.packets_rx.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_counter("voice.rtp.packets_tx_total")
            .with_description("RTP packets transmitted")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.packets_tx.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_counter("voice.rtp.nacks_total")
            .with_description("NACKs sent or received")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.nacks.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_counter("voice.rtp.plis_total")
            .with_description("PLIs sent or received")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.plis.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_counter("voice.rtp.firs_total")
            .with_description("FIRs sent or received")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.firs.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_counter("voice.udp.send_errors_total")
            .with_description("UDP send errors")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.udp_send_errors.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .u64_observable_gauge("voice.bwe_tx_bps")
            .with_description("Estimated egress bandwidth in bits per second")
            .with_callback(move |observer| {
                observer.observe(metrics.sum(|s| s.bwe_tx_bps.load(Relaxed)), &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .f64_observable_gauge("voice.rtt_ms")
            .with_description("Mean peer round-trip time in milliseconds")
            .with_callback(move |observer| {
                let rtt = metrics.mean(
                    |s| s.rtt_us_sum.load(Relaxed),
                    |s| s.rtt_count.load(Relaxed),
                );
                observer.observe(rtt / 1000.0, &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .f64_observable_gauge("voice.loss_fraction")
            .with_description("Mean packet loss fraction")
            .with_callback(move |observer| {
                let loss = metrics.mean(
                    |s| s.loss_ppm_sum.load(Relaxed),
                    |s| s.loss_count.load(Relaxed),
                );
                observer.observe(loss / 1_000_000.0, &[]);
            })
            .build();

        let metrics = Arc::clone(self);
        let _ = meter
            .f64_observable_gauge("voice.jitter")
            .with_description("Mean RTP interarrival jitter")
            .with_callback(move |observer| {
                observer.observe(
                    metrics.mean(
                        |s| s.jitter_sum.load(Relaxed),
                        |s| s.jitter_count.load(Relaxed),
                    ),
                    &[],
                );
            })
            .build();
    }

    fn lock_shards(&self) -> MutexGuard<'_, Vec<Arc<ShardMetrics>>> {
        self.shards.lock().unwrap_or_else(|err| err.into_inner())
    }

    fn sum(&self, value: impl Fn(&ShardMetrics) -> u64) -> u64 {
        self.lock_shards().iter().map(|s| value(s)).sum()
    }

    fn mean(
        &self,
        sum: impl Fn(&ShardMetrics) -> u64,
        count: impl Fn(&ShardMetrics) -> u64,
    ) -> f64 {
        let shards = self.lock_shards();
        let total: u64 = shards.iter().map(|s| sum(s)).sum();
        let observations: u64 = shards.iter().map(|s| count(s)).sum();
        if observations == 0 {
            0.0
        } else {
            total as f64 / observations as f64
        }
    }
}

/// per-peer cumulative counters used to derive deltas for shard metrics
#[derive(Default)]
pub struct PeerStatsState {
    bytes_rx: u64,
    bytes_tx: u64,
    bwe_tx: u64,
    ingress: HashMap<(Mid, Option<Rid>), MediaCounters>,
    egress: HashMap<(Mid, Option<Rid>), MediaCounters>,
}

#[derive(Default)]
struct MediaCounters {
    packets: u64,
    nacks: u64,
    plis: u64,
    firs: u64,
}

impl PeerStatsState {
    pub fn bwe_tx(&self) -> u64 {
        self.bwe_tx
    }

    pub fn peer_stats(&mut self, stats: &PeerStats, metrics: &ShardMetrics) {
        metrics
            .bytes_rx
            .fetch_add(stats.bytes_rx.saturating_sub(self.bytes_rx), Relaxed);
        metrics
            .bytes_tx
            .fetch_add(stats.bytes_tx.saturating_sub(self.bytes_tx), Relaxed);
        self.bytes_rx = stats.bytes_rx;
        self.bytes_tx = stats.bytes_tx;

        if let Some(bwe_tx) = stats.bwe_tx {
            let bwe_tx = bwe_tx.as_u64();
            metrics.update_bwe(self.bwe_tx, bwe_tx);
            self.bwe_tx = bwe_tx;
        }

        if let Some(rtt) = stats.rtt {
            metrics.observe_rtt(rtt);
        }
        if let Some(loss) = stats.egress_loss_fraction {
            metrics.observe_loss(loss);
        }
        if let Some(loss) = stats.ingress_loss_fraction {
            metrics.observe_loss(loss);
        }
    }

    pub fn media_ingress(&mut self, stats: &MediaIngressStats, metrics: &ShardMetrics) {
        let counters = self.ingress.entry((stats.mid, stats.rid)).or_default();

        metrics
            .packets_rx
            .fetch_add(stats.packets.saturating_sub(counters.packets), Relaxed);
        metrics
            .nacks
            .fetch_add(stats.nacks.saturating_sub(counters.nacks), Relaxed);
        metrics
            .plis
            .fetch_add(stats.plis.saturating_sub(counters.plis), Relaxed);
        metrics
            .firs
            .fetch_add(stats.firs.saturating_sub(counters.firs), Relaxed);
        metrics.observe_jitter(stats.jitter);

        counters.packets = stats.packets;
        counters.nacks = stats.nacks;
        counters.plis = stats.plis;
        counters.firs = stats.firs;
    }

    pub fn media_egress(&mut self, stats: &MediaEgressStats, metrics: &ShardMetrics) {
        let counters = self.egress.entry((stats.mid, stats.rid)).or_default();

        metrics
            .packets_tx
            .fetch_add(stats.packets.saturating_sub(counters.packets), Relaxed);
        metrics
            .nacks
            .fetch_add(stats.nacks.saturating_sub(counters.nacks), Relaxed);
        metrics
            .plis
            .fetch_add(stats.plis.saturating_sub(counters.plis), Relaxed);
        metrics
            .firs
            .fetch_add(stats.firs.saturating_sub(counters.firs), Relaxed);

        counters.packets = stats.packets;
        counters.nacks = stats.nacks;
        counters.plis = stats.plis;
        counters.firs = stats.firs;
    }
}

#[cfg(test)]
mod tests {
    use str0m::{Event as SEvent, bwe::Bitrate};

    use super::*;

    #[test]
    fn peer_stats_apply_deltas_once() {
        let metrics = ShardMetrics::default();
        let mut stats = PeerStatsState::default();
        let event = SEvent::PeerStats(PeerStats {
            peer_bytes_rx: 0,
            peer_bytes_tx: 0,
            bytes_rx: 100,
            bytes_tx: 40,
            timestamp: std::time::Instant::now(),
            bwe_tx: Some(Bitrate::bps(1000)),
            egress_loss_fraction: None,
            ingress_loss_fraction: None,
            rtt: None,
            selected_candidate_pair: None,
        });

        let SEvent::PeerStats(peer_stats) = &event else {
            unreachable!();
        };
        stats.peer_stats(peer_stats, &metrics);
        stats.peer_stats(peer_stats, &metrics);

        assert_eq!(metrics.bytes_rx.load(Relaxed), 100);
        assert_eq!(metrics.bytes_tx.load(Relaxed), 40);
        assert_eq!(metrics.bwe_tx_bps.load(Relaxed), 1000);
    }

    #[test]
    fn peer_removal_clears_bwe() {
        let metrics = ShardMetrics::default();
        metrics.peer_created();
        metrics.update_bwe(0, 5000);
        metrics.peer_removed(5000);

        assert_eq!(metrics.peers.load(Relaxed), 0);
        assert_eq!(metrics.bwe_tx_bps.load(Relaxed), 0);
    }
}
