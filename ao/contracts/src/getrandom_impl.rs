/// Custom getrandom implementation for WASM environment.
///
/// AO processes don't have access to OS-level randomness.
/// For proxy re-encryption (umbral-pre), randomness is needed for kFrag generation.
/// In production, entropy should be injected via message parameters.
///
/// TODO: Replace with a proper entropy injection mechanism.
///   Option 1: Accept entropy as a message field (client provides random bytes)
///   Option 2: Use AO's block hash as entropy seed
///   Option 3: Use a PRNG seeded from process ID + message ID
getrandom::register_custom_getrandom!(ao_getrandom);

pub fn ao_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    // Deterministic placeholder - NOT cryptographically secure
    // TODO: Replace with entropy from message context
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = (i as u8).wrapping_mul(7).wrapping_add(42);
    }
    Ok(())
}
