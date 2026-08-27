use nex_core::apps::discovery::*;

#[test]
fn test_r62_3_a_single_word_search() {
    let mut search = InvertedSearchIndex::new();
    let doc1 = [0x01u8; 32];
    let doc2 = [0x02u8; 32];

    search.index_document(doc1, "Sovereign decentralized cloud storage");
    search.index_document(doc2, "Offline vector maps and GPS navigation");

    let results = search.search("storage");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0], doc1);

    let map_results = search.search("maps");
    assert_eq!(map_results.len(), 1);
    assert_eq!(map_results[0], doc2);
}

#[test]
fn test_r62_3_b_multi_word_query_ranking() {
    let mut search = InvertedSearchIndex::new();
    let doc1 = [0x01u8; 32];
    let doc2 = [0x02u8; 32];

    search.index_document(doc1, "Rust core crypto runtime engine");
    search.index_document(doc2, "Rust network engine");

    let results = search.search("Rust crypto runtime");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], doc1, "Doc 1 matches 3 words so it must rank higher");
    assert_eq!(results[1], doc2, "Doc 2 matches 1 word");
}

#[test]
fn test_r62_3_c_punctuation_and_case_normalization() {
    let mut search = InvertedSearchIndex::new();
    let doc = [0x05u8; 32];

    search.index_document(doc, "Hello, World! This is a test... (Sovereign).");

    assert_eq!(search.search("hello"), vec![doc]);
    assert_eq!(search.search("world"), vec![doc]);
    assert_eq!(search.search("SOVEREIGN"), vec![doc]);
}

#[test]
fn test_r62_3_d_empty_query_returns_empty() {
    let mut search = InvertedSearchIndex::new();
    let doc = [0x01u8; 32];
    search.index_document(doc, "Some content");

    assert_eq!(search.search(""), Vec::<[u8; 32]>::new());
    assert_eq!(search.search("   "), Vec::<[u8; 32]>::new());
    assert_eq!(search.search("???"), Vec::<[u8; 32]>::new());
}

#[test]
fn test_r62_3_e_high_volume_indexing() {
    let mut search = InvertedSearchIndex::new();
    for i in 0..1000 {
        let doc = [i as u8; 32];
        search.index_document(doc, &format!("document number {} unique_tag_{}", i, i % 10));
    }

    let results = search.search("unique_tag_3");
    assert_eq!(results.len(), 100);
}

#[test]
fn test_r62_3_f_zero_regression_search_lifecycle() {
    let mut search = InvertedSearchIndex::new();
    for i in 0..10 {
        let doc = [i; 32];
        search.index_document(doc, "universal keyword");
    }
    assert_eq!(search.search("universal").len(), 10);
}
