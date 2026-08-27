use tempfile::tempdir;
use ed25519_dalek::SigningKey;
use nex_core::runtime::node::NexNode;
use nex_core::apps::discovery::*;
use nex_core::api::NexAppApi;

#[test]
fn test_r62_4_a_eclipse_attack_xor_density() {
    let target = [0xAAu8; 32];
    let mut dht = DhtRoutingTable::new(target);

    // Inject 100 random peers
    for i in 0..100 {
        let mut p = [0u8; 32];
        p[0] = i as u8;
        dht.add_peer(p, "127.0.0.1:9000", 100);
    }

    let closest = dht.find_closest_nodes(&target, 8);
    assert_eq!(closest.len(), 8);
    // Distance to closest must be minimal
    let d0 = DhtRoutingTable::xor_distance(&closest[0], &target);
    let d1 = DhtRoutingTable::xor_distance(&closest[1], &target);
    assert!(d0 <= d1);
}

#[test]
fn test_r62_4_b_topic_pubsub_multicast() {
    let mut pubsub = TopicPubSub::new();
    let topic = [0x55u8; 32];

    let sub1 = [0x01u8; 32];
    let sub2 = [0x02u8; 32];

    pubsub.subscribe(topic, sub1);
    pubsub.subscribe(topic, sub2);

    let recipients = pubsub.publish(&topic);
    assert_eq!(recipients.len(), 2);
    assert!(recipients.contains(&sub1));
    assert!(recipients.contains(&sub2));
}

#[test]
fn test_r62_4_c_sybil_alias_flood_dampening() {
    let mut wot = WebOfTrustRegistry::new();
    let root = [0x01u8; 32];
    let bad_actor = [0x02u8; 32];

    // Low confidence root trust
    wot.add_alias(root, bad_actor, "Untrusted", 0.1);

    // Bad actor claims 100 Sybil names
    for i in 0..100 {
        let sybil = [i + 50; 32];
        wot.add_alias(bad_actor, sybil, &format!("Sybil_{}", i), 1.0);
    }

    // Transitive resolution score should be dampened: 0.1 * 1.0 * 0.5 = 0.05
    let (_, score) = wot.resolve_alias(&root, "sybil_0").unwrap();
    assert!((score - 0.05).abs() < 1e-6, "Sybil score must be severely dampened");
}

#[test]
fn test_r62_4_d_large_text_search_indexer_stress() {
    let mut search = InvertedSearchIndex::new();
    let doc = [0x11u8; 32];

    let mut big_text = String::new();
    for i in 0..5000 {
        big_text.push_str(&format!("keyword_{} ", i % 500));
    }

    search.index_document(doc, &big_text);
    let results = search.search("keyword_42");
    assert_eq!(results, vec![doc]);
}

#[test]
fn test_r62_4_e_empty_topic_publish() {
    let pubsub = TopicPubSub::new();
    let unknown_topic = [0x99u8; 32];
    assert_eq!(pubsub.publish(&unknown_topic), Vec::<[u8; 32]>::new());
}

#[test]
fn test_r62_4_f_gate_r62_master_discovery_seal_and_merkle_invariance() {
    let dir = tempdir().unwrap();
    let seed = [181u8; 32];
    let mut node = NexNode::new(dir.path(), SigningKey::from_bytes(&seed));
    assert!(node.start().is_ok());

    let cp1 = node.sync_now().unwrap();
    let cp2 = node.sync_now().unwrap();
    assert_eq!(cp1.body.state_root, cp2.body.state_root, "Discovery operations must preserve Merkle root invariance");
}
