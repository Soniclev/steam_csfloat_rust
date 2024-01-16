use crate::fee::SteamFee;

#[test]
fn test_add_fee() {
    assert_eq!(SteamFee::add_fee(1), 3);
    assert_eq!(SteamFee::add_fee(9), 11);
    assert_eq!(SteamFee::add_fee(18), 20);
    assert_eq!(SteamFee::add_fee(19), 21);
    assert_eq!(SteamFee::add_fee(20), 23);
    assert_eq!(SteamFee::add_fee(59), 66);
    assert_eq!(SteamFee::add_fee(60), 69);
    assert_eq!(SteamFee::add_fee(130), 149);
    assert_eq!(SteamFee::add_fee(200), 230);
    assert_eq!(SteamFee::add_fee(300), 345);
    assert_eq!(SteamFee::add_fee(400), 460);
    assert_eq!(SteamFee::add_fee(500), 575);
    assert_eq!(SteamFee::add_fee(1243), 1429);
    assert_eq!(SteamFee::add_fee(12943), 14884);
}

#[test]
fn test_subtract_fee() {
    assert_eq!(SteamFee::subtract_fee(3), 1);
    assert_eq!(SteamFee::subtract_fee(4), 2);
    assert_eq!(SteamFee::subtract_fee(23), 20);
    assert_eq!(SteamFee::subtract_fee(22), 19);
    assert_eq!(SteamFee::subtract_fee(21), 19);
    assert_eq!(SteamFee::subtract_fee(20), 18);
    assert_eq!(SteamFee::subtract_fee(19), 17);
    assert_eq!(SteamFee::subtract_fee(149), 130);
    assert_eq!(SteamFee::subtract_fee(230), 200);
    assert_eq!(SteamFee::subtract_fee(345), 300);
    assert_eq!(SteamFee::subtract_fee(460), 400);
    assert_eq!(SteamFee::subtract_fee(575), 500);
    assert_eq!(SteamFee::subtract_fee(1429), 1243);
    assert_eq!(SteamFee::subtract_fee(2274), 1979);
    assert_eq!(SteamFee::subtract_fee(2484), 2160);
    assert_eq!(SteamFee::subtract_fee(14884), 12943);
    assert_eq!(SteamFee::subtract_fee(200000), 173914);
}
