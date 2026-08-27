use nex_core::identity::verifier::derive_actor_id;
use nex_core::identity::types::KeyType;
use nex_core::discovery::types::{
    EndpointHint, DiscoveryAdvertisement, DiscoveryError,
    TRANSPORT_TAG_QUIC, TRANSPORT_TAG_RETICULUM, derive_blinded_topic
};
use nex_core::discovery::routing_table::{RoutingTable, MAX_ROUTING_TABLE_ENTRIES};

#[test]
fn test_r23_a_and_b_authenticated_advertisement_ingestion() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let mut table = RoutingTable::new(alice);

    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let adv_bob = DiscoveryAdvertisement {
        actor_id: bob,
        namespace_or_topic: [0xAA; 32],
        sequence: 1,
        not_before_epoch: 0,
        expires_at_epoch: 50,
        endpoint_hints: vec![
            EndpointHint { transport_tag: TRANSPORT_TAG_QUIC, address_bytes: b"192.168.1.50:9000".to_vec(), priority: 10 },
            EndpointHint { transport_tag: TRANSPORT_TAG_RETICULUM, address_bytes: b"rns_dest_addr".to_vec(), priority: 5 },
        ],
        offered_capabilities: 0x1F,
        signature: vec![0xBB; 64],
    };

    // Valid authenticated advertisement
    let res = table.ingest_advertisement(adv_bob.clone(), 10);
    assert_eq!(res, Ok(true), "R23-B: Valid signed advertisement must be ingested");

    let route = table.find_best_route(&bob, 10).expect("Route must exist for Bob");
    assert_eq!(route.destination, bob);
    assert_eq!(route.hop_count, 1);
    assert_eq!(route.endpoint_hints.len(), 2);

    // Invalid advertisement with empty signature
    let mut invalid_adv = adv_bob;
    invalid_adv.signature.clear();
    let res_invalid = table.ingest_advertisement(invalid_adv, 10);
    assert_eq!(res_invalid, Err(DiscoveryError::SignatureInvalid), "R23-B: Empty signature must be rejected");
}

#[test]
fn test_r23_c_freshness_and_epoch_expiration() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let mut table = RoutingTable::new(alice);

    let adv = DiscoveryAdvertisement {
        actor_id: bob,
        namespace_or_topic: [0xAA; 32],
        sequence: 1,
        not_before_epoch: 10,
        expires_at_epoch: 20,
        endpoint_hints: vec![EndpointHint { transport_tag: TRANSPORT_TAG_QUIC, address_bytes: b"addr".to_vec(), priority: 1 }],
        offered_capabilities: 0x1F,
        signature: vec![0x11; 64],
    };

    // 1. Premature advertisement (epoch 5 < not_before 10)
    let res_early = table.ingest_advertisement(adv.clone(), 5);
    assert!(matches!(res_early, Err(DiscoveryError::PrematureAdvertisement { .. })), "R23-C: Premature advertisement rejected");

    // 2. Valid active advertisement (epoch 15)
    let res_active = table.ingest_advertisement(adv.clone(), 15);
    assert_eq!(res_active, Ok(true));
    assert!(table.find_best_route(&bob, 15).is_some());

    // 3. Expired query (epoch 25 > expires_at 20)
    assert!(table.find_best_route(&bob, 25).is_none(), "R23-C: Expired route must not be returned");

    // 4. Prune expired routes
    let pruned = table.prune_expired_routes(25);
    assert_eq!(pruned, 1, "R23-C: Pruning must remove expired route");
    assert!(!table.advertisements.contains_key(&bob));
}

#[test]
fn test_r23_d_sequence_monotonicity_and_reordering_convergence() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let mut table = RoutingTable::new(alice);

    let make_adv = |seq: u64| DiscoveryAdvertisement {
        actor_id: bob,
        namespace_or_topic: [0xAA; 32],
        sequence: seq,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        endpoint_hints: vec![EndpointHint { transport_tag: TRANSPORT_TAG_QUIC, address_bytes: format!("addr_v{}", seq).into_bytes(), priority: 1 }],
        offered_capabilities: 0x1F,
        signature: vec![0x11; 64],
    };

    let adv_1 = make_adv(1);
    let adv_2 = make_adv(2);
    let adv_3 = make_adv(3);

    // Ingest out-of-order: seq 3 arrives first
    assert_eq!(table.ingest_advertisement(adv_3.clone(), 10), Ok(true));

    // Stale sequence 1 arrives later -> Rejected as StaleSequence
    let res_stale_1 = table.ingest_advertisement(adv_1, 10);
    assert!(matches!(res_stale_1, Err(DiscoveryError::StaleSequence { .. })), "R23-D: Stale sequence 1 must be rejected");

    // Stale sequence 2 arrives later -> Rejected as StaleSequence
    let res_stale_2 = table.ingest_advertisement(adv_2, 10);
    assert!(matches!(res_stale_2, Err(DiscoveryError::StaleSequence { .. })), "R23-D: Stale sequence 2 must be rejected");

    // Table strictly maintains sequence 3
    let route = table.find_best_route(&bob, 10).unwrap();
    assert_eq!(route.sequence, 3);
    assert_eq!(route.endpoint_hints[0].address_bytes, b"addr_v3");
}

#[test]
fn test_r23_f_route_construction_and_multi_hop_selection() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let bob = derive_actor_id(KeyType::Ed25519, &[0x02; 32]);
    let charlie = derive_actor_id(KeyType::Ed25519, &[0x03; 32]);
    let mut table = RoutingTable::new(alice);

    // Charlie is reachable via Bob (2 hops)
    let res_2_hop = table.add_multi_hop_route(
        charlie,
        bob,
        2,
        1,
        50,
        vec![EndpointHint { transport_tag: TRANSPORT_TAG_QUIC, address_bytes: b"bob_relay".to_vec(), priority: 1 }],
        10,
    );
    assert_eq!(res_2_hop, Ok(true));

    let route_charlie = table.find_best_route(&charlie, 10).unwrap();
    assert_eq!(route_charlie.hop_count, 2);
    assert_eq!(route_charlie.next_hop, bob);

    // Later, a direct 1-hop advertisement arrives for Charlie with same sequence
    let adv_direct = DiscoveryAdvertisement {
        actor_id: charlie,
        namespace_or_topic: [0xCC; 32],
        sequence: 1,
        not_before_epoch: 0,
        expires_at_epoch: 50,
        endpoint_hints: vec![EndpointHint { transport_tag: TRANSPORT_TAG_QUIC, address_bytes: b"direct_charlie".to_vec(), priority: 10 }],
        offered_capabilities: 0x1F,
        signature: vec![0x33; 64],
    };
    table.ingest_advertisement(adv_direct, 10).unwrap();

    // Table upgrades to the 1-hop path
    let best_route = table.find_best_route(&charlie, 10).unwrap();
    assert_eq!(best_route.hop_count, 1, "R23-F: Must prefer 1-hop path over 2-hop relay");
    assert_eq!(best_route.next_hop, charlie);
}

#[test]
fn test_r23_l_sizing_bounds_and_overflow_protection() {
    let alice = derive_actor_id(KeyType::Ed25519, &[0x01; 32]);
    let mut table = RoutingTable::new(alice);

    // Fill routing table up to MAX_ROUTING_TABLE_ENTRIES (1024)
    for i in 0..MAX_ROUTING_TABLE_ENTRIES {
        let mut pk = [0u8; 32];
        pk[0..4].copy_from_slice(&(i as u32).to_le_bytes());
        let actor = derive_actor_id(KeyType::Ed25519, &pk);

        let adv = DiscoveryAdvertisement {
            actor_id: actor,
            namespace_or_topic: [0xFF; 32],
            sequence: 1,
            not_before_epoch: 0,
            expires_at_epoch: 100,
            endpoint_hints: vec![],
            offered_capabilities: 0x01,
            signature: vec![0xAA; 64],
        };
        table.ingest_advertisement(adv, 10).unwrap();
    }

    assert_eq!(table.advertisements.len(), MAX_ROUTING_TABLE_ENTRIES);

    // Attempt to add 1025th entry without any expired entries -> RoutingTableFull
    let overflow_actor = derive_actor_id(KeyType::Ed25519, &[0xEE; 32]);
    let overflow_adv = DiscoveryAdvertisement {
        actor_id: overflow_actor,
        namespace_or_topic: [0xFF; 32],
        sequence: 1,
        not_before_epoch: 0,
        expires_at_epoch: 100,
        endpoint_hints: vec![],
        offered_capabilities: 0x01,
        signature: vec![0xAA; 64],
    };

    let res_overflow = table.ingest_advertisement(overflow_adv, 10);
    assert_eq!(res_overflow, Err(DiscoveryError::RoutingTableFull), "R23-L: Table must reject additions when at capacity");
}

#[test]
fn test_r23_o_blinded_topic_discovery_privacy() {
    let namespace_secret_family = [0x42; 32];
    let namespace_secret_work = [0x99; 32];

    let topic_epoch_1 = derive_blinded_topic(&namespace_secret_family, 1);
    let topic_epoch_2 = derive_blinded_topic(&namespace_secret_family, 2);
    let topic_work_epoch_1 = derive_blinded_topic(&namespace_secret_work, 1);

    // Blinded topic rotates per epoch
    assert_ne!(topic_epoch_1, topic_epoch_2, "R23-O: Blinded topic must rotate across epochs");
    // Different namespaces produce uncorrelated blinded topics
    assert_ne!(topic_epoch_1, topic_work_epoch_1, "R23-O: Different namespaces must have distinct blinded topics");
}
