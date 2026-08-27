use std::fs::File;
use std::io::Read;
use serde::Deserialize;
use serde_json::Value;
use nex_core::{resolve_genesis_collision, validate_identity_genesis_authority, HashRef};

#[derive(Deserialize)]
struct ConformanceSuite {
    version: String,
    description: String,
    vectors: Vec<Value>,
}

#[test]
fn test_conformance_vectors() {
    let mut file = File::open("../reference/conformance_suite_v1.0_final_independent.json")
        .expect("Failed to open conformance suite JSON");
    
    let mut data = String::new();
    file.read_to_string(&mut data).expect("Failed to read JSON");
    
    let suite: ConformanceSuite = serde_json::from_str(&data).expect("Failed to parse JSON");
    println!("Running {}...", suite.version);
    
    for vector in suite.vectors {
        let test_name = vector["test_name"].as_str().unwrap();
        
        match test_name {
            "Genesis_Collision_Determinism_Min_Lex" => {
                let c1_hex = vector["inputs"]["candidates"][0].as_str().unwrap();
                let c2_hex = vector["inputs"]["candidates"][1].as_str().unwrap();
                
                let h1 = HashRef { algorithm_id: 1, digest: hex::decode(c1_hex).unwrap() };
                let h2 = HashRef { algorithm_id: 1, digest: hex::decode(c2_hex).unwrap() };
                
                let expected_winner_hex = vector["expected_winner_hex"].as_str().unwrap();
                
                let winner = resolve_genesis_collision(&[h1, h2]).unwrap();
                assert_eq!(hex::encode(&winner.digest), expected_winner_hex);
                println!("✅ Passed: {}", test_name);
            },
            "Identity_Genesis_Valid_Authority" | "Identity_Genesis_Invalid_Authority" => {
                let author_key = hex::decode(vector["inputs"]["author_key"].as_str().unwrap()).unwrap();
                let root_key = hex::decode(vector["inputs"]["root_key"].as_str().unwrap()).unwrap();
                let ctx_nil = vector["inputs"]["ctx_is_nil"].as_bool().unwrap();
                let cap_nil = vector["inputs"]["cap_is_nil"].as_bool().unwrap();
                
                let result = validate_identity_genesis_authority(&author_key, &root_key, ctx_nil, cap_nil);
                let expected_state = vector["expected_authority_state"].as_str().unwrap();
                
                let string_result = match result {
                    Ok(_) => "VALID",
                    Err(e) => e,
                };
                
                assert_eq!(string_result, expected_state);
                println!("✅ Passed: {}", test_name);
            },
            _ => {
                println!("⚠️ Skipping unmapped test: {}", test_name);
            }
        }
    }
}
