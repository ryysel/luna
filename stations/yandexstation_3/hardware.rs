//! Station 3 ring: 36 RGB LEDs behind sysfs nodes.
//!
//! Every LED has `red`/`green`/`blue` brightness nodes under
//! `/sys/class/leds/orion:<channel>:ring<index>`, which is also what the
//! firmware and the QEMU hardware bridge read to drive the ring.

use anyhow::{bail, Context, Result};
use luna::LedBackend;
use std::fs;

const RING_COUNT: usize = 36;
const PREFIX: &str = "orion";

pub fn create_led_backend() -> Result<Box<dyn LedBackend>> {
    Ok(Box::new(SysfsRing))
}

struct SysfsRing;

impl LedBackend for SysfsRing {
    fn show(&self, frame: &[u8]) -> Result<()> {
        if frame.len() != RING_COUNT * 3 {
            bail!(
                "unexpected Station 3 ring frame length {}",
                frame.len()
            );
        }
        for index in 0..RING_COUNT {
            for (channel, value) in ["red", "green", "blue"]
                .iter()
                .zip(&frame[index * 3..index * 3 + 3])
            {
                let path = format!("/sys/class/leds/{PREFIX}:{channel}:ring{index}/brightness");
                fs::write(&path, value.to_string()).with_context(|| format!("write {path}"))?;
            }
        }
        Ok(())
    }
}
