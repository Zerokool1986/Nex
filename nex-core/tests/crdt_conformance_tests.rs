use std::fs::File;
use std::io::Read;
use serde::Deserialize;
use serde_json::Value;
use nex_core::crdt::evaluator::{CrdtEvaluator, EvaluationItem};
use nex_core::crdt::types::{OperationBody, OperationIndex};
use nex_core::crdt::topology::{Epoch, LamportRank};
use nex_core::crdt::optag::compute_optag;
use nex_core::crdt::projection::project;
use nex_core::HashRef;

#[derive(Deserialize)]
struct CrdtSuite {
    version: String,
    description: String,
    vectors: Vec<Value>,
}

fn parse_hashref(hex_str: &str) -> HashRef {
    let bytes = hex::decode(hex_str).unwrap();
    // Assuming the wire format has an alg id of some sort, but let's just parse the digest directly if it's 32 bytes or skip prefix
    // The test JSON gives mutation_id like "82015820..." which is CBOR.
    // Let's just use it as raw bytes for the HashRef for now, or parse properly if it's CBOR.
    // Actually, it says mutation_id: "820158201111111111111111111111111111111111111111111111111111111111111111"
    // 82 (array of 2) 01 (alg 1) 58 20 (bytes 32) ...
    if bytes.len() == 36 && bytes[0] == 0x82 && bytes[1] == 0x01 && bytes[2] == 0x58 && bytes[3] == 0x20 {
        HashRef {
            algorithm_id: 1,
            digest: bytes[4..].to_vec(),
        }
    } else {
        HashRef {
            algorithm_id: 1, // Assume 1
            digest: bytes,
        }
    }
}

#[test]
fn test_crdt_vectors() {
    let mut file = File::open("d:/Nex/reference/conformance_suite_v1.1_crdt_authoritative.json")
        .expect("Failed to open CRDT conformance JSON");
    
    let mut data = String::new();
    file.read_to_string(&mut data).expect("Failed to read JSON");
    
    let suite: CrdtSuite = serde_json::from_str(&data).expect("Failed to parse JSON");
    println!("Running {}...", suite.version);
    
    for vector in suite.vectors {
        let test_name = vector["test_name"].as_str().unwrap();
        
        let mut items = Vec::new();
        if let Some(ops) = vector["inputs"]["operations"].as_array() {
            for op in ops {
                let mutation_id = parse_hashref(op["mutation_id"].as_str().unwrap());
                let op_index = OperationIndex(op["op_index"].as_u64().unwrap() as u32);
                let epoch = Epoch(op["epoch"].as_u64().unwrap());
                let lamport_rank = LamportRank(op["lamport"].as_u64().unwrap());
                
                let typ = op["type"].as_str().unwrap();
                let body = match typ {
                    "ADD" => OperationBody::Add {
                        key: op["key"].as_str().unwrap().as_bytes().to_vec(),
                        payload: op["payload"].as_str().unwrap().as_bytes().to_vec(),
                    },
                    "REMOVE" => OperationBody::Remove {
                        key: op["key"].as_str().unwrap().as_bytes().to_vec(),
                    },
                    "RESURRECT" => OperationBody::Resurrect,
                    "INIT_OBJECT" => OperationBody::InitObject,
                    _ => panic!("Unknown type"),
                };
                
                let op_tag = compute_optag(&mutation_id, &op_index, &body);
                
                items.push(EvaluationItem {
                    epoch,
                    lamport_rank,
                    mutation_id,
                    operation_index: op_index,
                    body,
                    op_tag,
                });
            }
        }
        
        let mut evaluator = CrdtEvaluator::new();
        evaluator.evaluate(items);
        
        let projection = project(&evaluator.register_state);
        
        let expected_adds = vector["expected_state_projection"]["AddsMap"].as_object().unwrap();
        assert_eq!(projection.adds_map.len(), expected_adds.len(), "AddsMap length mismatch");
        
        let expected_tombstones = vector["expected_state_projection"]["TombstonesArr"].as_array().unwrap();
        assert_eq!(projection.tombstones_arr.len(), expected_tombstones.len(), "TombstonesArr length mismatch");
        
        println!("✅ Passed: {}", test_name);
    }
}
