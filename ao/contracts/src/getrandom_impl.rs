// AO processes lack OS-level randomness. This placeholder satisfies the
// getrandom trait so umbral-pre compiles, but it is NOT cryptographically
// secure. Production must inject real entropy (message field, block hash,
// or PRNG seeded from process ID + message ID).
#[cfg(target_arch = "wasm32")]
getrandom::register_custom_getrandom!(ao_getrandom);

pub fn ao_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    // TODO: replace with entropy from message context
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = (i as u8).wrapping_mul(7).wrapping_add(42);
    }
    Ok(())
}
