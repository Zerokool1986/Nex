use nex_core::sync::relay::{RelayStore, RelayEnvelope};

#[test]
fn test_r71_19_a_relay_store_buffer_and_drain() {
    let mut store = RelayStore::new();

    let recipient = [0xAA; 32];
    let env = RelayEnvelope {
        relay_session_id: [0x01; 16],
        sender_actor: [0x11; 32],
        recipient_actor: recipient,
        encrypted_payload: b"Opaque encrypted blob".to_vec(),
        ephemeral_nonce: [0x22; 16],
        expiration_epoch: 1000,
    };

    store.buffer_envelope(env.clone());
    assert_eq!(store.total_buffered_count(), 1);

    // Recipient connects and drains
    let drained = store.drain_for_recipient(&recipient, 500);
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].encrypted_payload, b"Opaque encrypted blob");
    assert_eq!(store.total_buffered_count(), 0);
}

#[test]
fn test_r71_19_b_expired_relay_envelope_filtered_out() {
    let mut store = RelayStore::new();

    let recipient = [0xBB; 32];
    let env = RelayEnvelope {
        relay_session_id: [0x02; 16],
        sender_actor: [0x22; 32],
        recipient_actor: recipient,
        encrypted_payload: b"Old envelope".to_vec(),
        ephemeral_nonce: [0x33; 16],
        expiration_epoch: 500, // Expires at epoch 500
    };

    store.buffer_envelope(env);

    // Attempt drain at epoch 600 > 500
    let drained = store.drain_for_recipient(&recipient, 600);
    assert!(drained.is_empty(), "Expired envelope must not be delivered");
}

#[test]
fn test_r71_19_c_multi_recipient_isolation() {
    let mut store = RelayStore::new();

    let r1 = [0x01; 32];
    let r2 = [0x02; 32];

    store.buffer_envelope(RelayEnvelope {
        relay_session_id: [0x11; 16],
        sender_actor: [0xAA; 32],
        recipient_actor: r1,
        encrypted_payload: b"Payload for R1".to_vec(),
        ephemeral_nonce: [0x00; 16],
        expiration_epoch: 1000,
    });

    store.buffer_envelope(RelayEnvelope {
        relay_session_id: [0x22; 16],
        sender_actor: [0xAA; 32],
        recipient_actor: r2,
        encrypted_payload: b"Payload for R2".to_vec(),
        ephemeral_nonce: [0x00; 16],
        expiration_epoch: 1000,
    });

    assert_eq!(store.total_buffered_count(), 2);

    let d1 = store.drain_for_recipient(&r1, 100);
    assert_eq!(d1.len(), 1);
    assert_eq!(d1[0].encrypted_payload, b"Payload for R1");

    assert_eq!(store.total_buffered_count(), 1);

    let d2 = store.drain_for_recipient(&r2, 100);
    assert_eq!(d2.len(), 1);
    assert_eq!(d2[0].encrypted_payload, b"Payload for R2");

    assert_eq!(store.total_buffered_count(), 0);
}

#[test]
fn test_r71_19_d_empty_drain_is_noop() {
    let mut store = RelayStore::new();
    let unknown_actor = [0x99; 32];
    assert!(store.drain_for_recipient(&unknown_actor, 100).is_empty());
}

#[test]
fn test_r71_19_e_multiple_envelopes_for_same_recipient() {
    let mut store = RelayStore::new();
    let recipient = [0xCC; 32];

    for i in 1..=5 {
        store.buffer_envelope(RelayEnvelope {
            relay_session_id: [i as u8; 16],
            sender_actor: [0x11; 32],
            recipient_actor: recipient,
            encrypted_payload: format!("Envelope {}", i).into_bytes(),
            ephemeral_nonce: [0x00; 16],
            expiration_epoch: 1000,
        });
    }

    assert_eq!(store.total_buffered_count(), 5);
    let drained = store.drain_for_recipient(&recipient, 100);
    assert_eq!(drained.len(), 5);
    assert_eq!(store.total_buffered_count(), 0);
}

#[test]
fn test_r71_19_f_opaque_relay_payload_preservation() {
    let mut store = RelayStore::new();
    let recipient = [0xDD; 32];
    let binary_data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF, 0x42];

    store.buffer_envelope(RelayEnvelope {
        relay_session_id: [0x55; 16],
        sender_actor: [0xEE; 32],
        recipient_actor: recipient,
        encrypted_payload: binary_data.clone(),
        ephemeral_nonce: [0x77; 16],
        expiration_epoch: 1000,
    });

    let drained = store.drain_for_recipient(&recipient, 100);
    assert_eq!(drained[0].encrypted_payload, binary_data);
}
