use nex_core::apps::discovery::*;

#[test]
fn test_r62_1_a_xor_distance_symmetry_and_identity() {
    let a = [0xAAu8; 32];
    let b = [0x55u8; 32];

    assert_eq!(DhtRoutingTable::xor_distance(&a, &a), [0u8; 32]);
    assert_eq!(DhtRoutingTable::xor_distance(&a, &b), DhtRoutingTable::xor_distance(&b, &a));
    assert_eq!(DhtRoutingTable::xor_distance(&a, &b), [0xFFu8; 32]);
}

#[test]
fn test_r62_1_b_peer_locator_insertion_and_closest_query() {
    let local = [0x00u8; 32];
    let mut dht = DhtRoutingTable::new(local);

    let p1 = [0x01u8; 32];
    let p2 = [0x02u8; 32];
    let p3 = [0xFFu8; 32];

    dht.add_peer(p1, "127.0.0.1:9001", 100);
    dht.add_peer(p2, "127.0.0.1:9002", 100);
    dht.add_peer(p3, "127.0.0.1:9003", 100);

    let target = [0x00u8; 32];
    let closest = dht.find_closest_nodes(&target, 2);

    assert_eq!(closest.len(), 2);
    assert_eq!(closest[0], p1);
    assert_eq!(closest[1], p2);
}

#[test]
fn test_r62_1_c_dht_20_node_routing_convergence() {
    let local = [0x00u8; 32];
    let mut dht = DhtRoutingTable::new(local);

    for i in 1..=20 {
        let mut p = [0u8; 32];
        p[31] = i as u8;
        dht.add_peer(p, &format!("10.0.0.{}:8000", i), 200);
    }

    assert_eq!(dht.peers.len(), 20);

    let target = [0x00u8; 32];
    let closest = dht.find_closest_nodes(&target, 5);
    assert_eq!(closest.len(), 5);
    assert_eq!(closest[0][31], 1);
    assert_eq!(closest[4][31], 5);
}

#[test]
fn test_r62_1_d_peer_locator_update() {
    let local = [0x00u8; 32];
    let mut dht = DhtRoutingTable::new(local);

    let peer = [0x10u8; 32];
    dht.add_peer(peer, "1.1.1.1:80", 100);
    assert_eq!(dht.peers.get(&peer).unwrap().socket_addr, "1.1.1.1:80");

    dht.add_peer(peer, "2.2.2.2:80", 200);
    assert_eq!(dht.peers.get(&peer).unwrap().socket_addr, "2.2.2.2:80");
    assert_eq!(dht.peers.get(&peer).unwrap().last_seen_epoch, 200);
}

#[test]
fn test_r62_1_e_closest_nodes_exceeding_count() {
    let local = [0x00u8; 32];
    let mut dht = DhtRoutingTable::new(local);

    let p1 = [0x01u8; 32];
    dht.add_peer(p1, "127.0.0.1:9000", 10);

    let closest = dht.find_closest_nodes(&[0u8; 32], 10);
    assert_eq!(closest.len(), 1);
}

#[test]
fn test_r62_1_f_zero_regression_dht_lifecycle() {
    let local = [0x77u8; 32];
    let mut dht = DhtRoutingTable::new(local);
    for i in 0..10 {
        let p = [i + 1; 32];
        dht.add_peer(p, "127.0.0.1:9000", 100);
    }
    assert_eq!(dht.peers.len(), 10);
}
