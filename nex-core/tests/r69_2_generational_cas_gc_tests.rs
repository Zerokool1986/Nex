use std::collections::BTreeSet;
use nex_core::storage::compactor::{CasCompactor, DEFAULT_TOMBSTONE_GRACE_EPOCHS};

#[test]
fn test_r69_2_a_active_chunks_retained_indefinitely() {
    let mut compactor = CasCompactor::new(DEFAULT_TOMBSTONE_GRACE_EPOCHS);
    let chunk1 = [0x01; 32];
    let chunk2 = [0x02; 32];

    compactor.insert_chunk(chunk1, 10, 4096);
    compactor.insert_chunk(chunk2, 10, 8192);

    let mut active = BTreeSet::new();
    active.insert(chunk1);
    active.insert(chunk2);

    let report = compactor.collect_garbage(100, &active);
    assert_eq!(report.chunks_reclaimed, 0);
    assert_eq!(report.remaining_active_bytes, 12288);
    assert_eq!(compactor.chunks.len(), 2);
}

#[test]
fn test_r69_2_b_tombstone_grace_period_protection() {
    let mut compactor = CasCompactor::new(30);
    let chunk1 = [0x11; 32];
    compactor.insert_chunk(chunk1, 10, 4096);

    // Tombstone at epoch 50
    compactor.mark_tombstone(&chunk1, 50);

    let empty_active = BTreeSet::new();

    // At epoch 65 (age = 15 < grace 30): must NOT be collected
    let report_early = compactor.collect_garbage(65, &empty_active);
    assert_eq!(report_early.chunks_reclaimed, 0);
    assert_eq!(compactor.chunks.len(), 1);

    // At epoch 80 (age = 30 >= grace 30): MUST be collected
    let report_mature = compactor.collect_garbage(80, &empty_active);
    assert_eq!(report_mature.chunks_reclaimed, 1);
    assert_eq!(report_mature.bytes_reclaimed, 4096);
    assert_eq!(compactor.chunks.len(), 0);
}

#[test]
fn test_r69_2_c_active_reference_overrides_stale_tombstone() {
    let mut compactor = CasCompactor::new(30);
    let chunk1 = [0x22; 32];
    compactor.insert_chunk(chunk1, 10, 4096);
    compactor.mark_tombstone(&chunk1, 20);

    // If an active SMT checkpoint still references chunk1 at epoch 100, do not purge
    let mut active = BTreeSet::new();
    active.insert(chunk1);

    let report = compactor.collect_garbage(100, &active);
    assert_eq!(report.chunks_reclaimed, 0);
    assert_eq!(compactor.chunks.len(), 1);
}

#[test]
fn test_r69_2_d_batch_compaction_reclaims_multi_megabyte_storage() {
    let mut compactor = CasCompactor::new(10);
    let total_chunks = 100;
    for i in 0..total_chunks {
        let mut hash = [0u8; 32];
        hash[0] = i as u8;
        compactor.insert_chunk(hash, 5, 64 * 1024); // 64 KB each = 6.4 MB total
        compactor.mark_tombstone(&hash, 10);
    }

    let empty_active = BTreeSet::new();
    let report = compactor.collect_garbage(25, &empty_active);

    assert_eq!(report.chunks_reclaimed, 100);
    assert_eq!(report.bytes_reclaimed, 100 * 64 * 1024);
    assert_eq!(compactor.chunks.len(), 0);
}

#[test]
fn test_r69_2_e_mixed_generation_compaction_isolation() {
    let mut compactor = CasCompactor::new(20);
    
    // Chunk A: Tombstoned at epoch 10
    let chunk_a = [0xAA; 32];
    compactor.insert_chunk(chunk_a, 5, 1000);
    compactor.mark_tombstone(&chunk_a, 10);

    // Chunk B: Tombstoned at epoch 40
    let chunk_b = [0xBB; 32];
    compactor.insert_chunk(chunk_b, 35, 2000);
    compactor.mark_tombstone(&chunk_b, 40);

    // Chunk C: Never tombstoned
    let chunk_c = [0xCC; 32];
    compactor.insert_chunk(chunk_c, 45, 3000);

    let empty_active = BTreeSet::new();

    // At epoch 35: Only Chunk A is >= 20 epochs old (35 - 10 = 25 >= 20)
    let report = compactor.collect_garbage(35, &empty_active);
    assert_eq!(report.chunks_reclaimed, 1);
    assert_eq!(report.bytes_reclaimed, 1000);
    assert_eq!(compactor.chunks.len(), 2);
    assert!(compactor.chunks.contains_key(&chunk_b));
    assert!(compactor.chunks.contains_key(&chunk_c));
}

#[test]
fn test_r69_2_f_zero_tombstone_idempotency() {
    let mut compactor = CasCompactor::new(30);
    let chunk1 = [0x99; 32];
    compactor.insert_chunk(chunk1, 10, 1024);

    let empty_active = BTreeSet::new();
    let report1 = compactor.collect_garbage(100, &empty_active);
    assert_eq!(report1.chunks_reclaimed, 0, "Untombstoned chunk must never be collected");

    let report2 = compactor.collect_garbage(200, &empty_active);
    assert_eq!(report2.chunks_reclaimed, 0);
    assert_eq!(compactor.chunks.len(), 1);
}
