// NOTE: maybe i can move this to crate-common/src/v1/types/voice/str0m.rs?

use common::v1::types::voice::TrackEncoding;
use str0m::media::{Simulcast, SimulcastLayer};

pub fn get_simulcast(encodings: &[TrackEncoding]) -> Simulcast {
    let mut sim = Simulcast::new();
    for enc in encodings {
        sim.add_send_layer(get_simulcast_layer(enc));
    }
    sim
}

pub fn get_simulcast_layer(encoding: &TrackEncoding) -> SimulcastLayer {
    // all resolutions are 16:9
    match encoding {
        TrackEncoding::Source => SimulcastLayer::new("s"),
        TrackEncoding::Full => SimulcastLayer::new_with_attributes("f")
            .max_width(1920)
            .max_height(1080)
            .max_br(6_000_000)
            .max_fps(60)
            .build(),
        TrackEncoding::Reduced => SimulcastLayer::new_with_attributes("r")
            .max_width(640)
            .max_height(360)
            .max_br(1_000_000)
            .max_fps(30)
            .build(),
        TrackEncoding::Thumbnail => SimulcastLayer::new_with_attributes("t")
            .max_width(320)
            .max_height(180)
            .max_br(150_000)
            .max_fps(15)
            .build(),
    }
}
