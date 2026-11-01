use super::*;

#[test]
fn ssr_needs_hdr_and_water_reflections() {
    const SSR_BIT: u32 = 1 << 10;
    let on = Effects::default();
    assert_ne!(on.bits() & SSR_BIT, 0);
    assert_eq!(Effects { hdr: false, ..on }.bits() & SSR_BIT, 0);
    assert_eq!(
        Effects {
            water_reflections: false,
            ..on
        }
        .bits()
            & SSR_BIT,
        0
    );
    assert_eq!(Effects { ssr: false, ..on }.bits() & SSR_BIT, 0);
}
